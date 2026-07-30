// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! Ratatui draw routines for the screener TUI.

use std::collections::HashSet;

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};
use ratatui::Frame;
use serde_json::Value;

use super::model::{AppModel, ViewMode};
use super::style::tone_color;
use crate::beautify::{format_cell_for_field, RowTechMap};
use crate::field::FieldDef;
use crate::util::get_columns_to_request;

const LABEL: Style = Style::new().fg(Color::DarkGray);
const ACCENT: Style = Style::new().fg(Color::Cyan);
const MUTED: Style = Style::new().fg(Color::Gray);

/// Draws the TUI chrome and active pane.
pub fn draw(frame: &mut Frame<'_>, model: &AppModel) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(2),
        ])
        .split(frame.area());

    draw_title(frame, chunks[0], model);
    match model.view {
        ViewMode::Results => draw_results(frame, chunks[1], model),
        ViewMode::Help => draw_help(frame, chunks[1]),
    }
    draw_footer(frame, chunks[2], model);
}

fn draw_title(frame: &mut Frame<'_>, area: Rect, model: &AppModel) {
    let preset = model.config.preset.as_deref().unwrap_or("(defaults)");
    let title = Paragraph::new(Line::from(vec![
        Span::styled("tvscreener-tui", ACCENT.add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(preset, MUTED),
    ]))
    .block(Block::default().borders(Borders::ALL).title(" Screener "));
    frame.render_widget(title, area);
}

fn draw_footer(frame: &mut Frame<'_>, area: Rect, model: &AppModel) {
    let status = model.error.as_deref().unwrap_or(model.status.as_str());
    let style = if model.error.is_some() {
        Style::new().fg(Color::Red)
    } else {
        MUTED
    };
    let line = Paragraph::new(Line::from(vec![
        Span::styled(status, style),
        Span::raw("  "),
        Span::styled(
            "keys: q quit · r refresh · a watch · h help · ↑↓ scroll",
            LABEL,
        ),
    ]));
    frame.render_widget(line, area);
}

fn draw_help(frame: &mut Frame<'_>, area: Rect) {
    let lines = vec![
        Line::from(Span::styled("Keys", ACCENT.add_modifier(Modifier::BOLD))),
        Line::from("  q / Esc / Ctrl-C   quit"),
        Line::from("  r                 refresh scan (min 10s between scans)"),
        Line::from("  a                 toggle watch (auto-refresh, default 30s)"),
        Line::from("  h                 toggle this help"),
        Line::from("  ↑ / ↓ / PgUp/Dn   scroll results"),
        Line::from(""),
        Line::from(Span::styled(
            "Refresh policy",
            ACCENT.add_modifier(Modifier::BOLD),
        )),
        Line::from("  Default is manual (one scan at start). Watch is opt-in."),
        Line::from("  TUI floor is 10s (library stream() floor is 1s; do not use that here)."),
        Line::from(""),
        Line::from(Span::styled(
            "Scaffold note",
            ACCENT.add_modifier(Modifier::BOLD),
        )),
        Line::from("  Builder + Payload JSON panes land in a follow-up PR."),
        Line::from("  Configure via CLI flags: asset, --preset, --limit, --watch, …"),
    ];
    let body = Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" Help "));
    frame.render_widget(body, area);
}

fn draw_results(frame: &mut Frame<'_>, area: Rect, model: &AppModel) {
    let selected = select_table_fields(&model.rows, &model.fields, model.max_columns);
    let columns = get_columns_to_request(&model.fields);

    let mut header_cells = vec![Cell::from("Symbol").style(ACCENT.add_modifier(Modifier::BOLD))];
    header_cells.extend(
        selected
            .iter()
            .map(|f| Cell::from(f.label.as_str()).style(ACCENT.add_modifier(Modifier::BOLD))),
    );
    let header = Row::new(header_cells).height(1);

    let visible = model.rows.get(model.scroll..).unwrap_or(&[]);
    let body_rows: Vec<Row<'_>> = visible
        .iter()
        .map(|row| {
            let tech = RowTechMap::from_row(row, &columns);
            let mut cells = vec![Cell::from(row.symbol.as_str())];
            for field in &selected {
                let value = row.data.get(&field.label).unwrap_or(&Value::Null);
                let cell = format_cell_for_field(value, field, Some(&tech));
                let style = Style::new().fg(tone_color(cell.tone));
                cells.push(Cell::from(cell.text).style(style));
            }
            Row::new(cells)
        })
        .collect();

    let mut widths = vec![Constraint::Length(22)];
    widths.extend(selected.iter().map(|_| Constraint::Min(10)));

    let table = Table::new(body_rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(" Results "))
        .column_spacing(1);

    frame.render_widget(table, area);
}

fn select_table_fields<'a>(
    rows: &[crate::ScreenerRow],
    fields: &'a [FieldDef],
    max_columns: usize,
) -> Vec<&'a FieldDef> {
    let present: HashSet<&str> = rows
        .iter()
        .flat_map(|r| r.data.keys().map(String::as_str))
        .collect();

    let mut selected: Vec<&FieldDef> = fields
        .iter()
        .filter(|f| !f.field_name.starts_with("candlestick"))
        .filter(|f| present.contains(f.label.as_str()))
        .take(max_columns.max(1))
        .collect();

    if selected.is_empty() {
        selected = fields
            .iter()
            .filter(|f| !f.field_name.starts_with("candlestick"))
            .take(max_columns.max(1))
            .collect();
    }
    selected
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::Asset;
    use crate::tui::model::{ScanConfig, DEFAULT_WATCH_INTERVAL_SECS};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use serde_json::json;

    fn sample_model() -> AppModel {
        let config = ScanConfig {
            asset: Asset::Crypto,
            preset: Some("crypto_price".into()),
            from: 0,
            limit: 5,
            search: None,
            markets: None,
            index: None,
        };
        let fields = vec![FieldDef {
            label: "Change %".into(),
            field_name: "change".into(),
            format: Some("percent".into()),
            interval: true,
            historical: false,
        }];
        let mut data = serde_json::Map::new();
        data.insert("Change %".into(), json!(1.25));
        let row = crate::ScreenerRow {
            symbol: "BINANCE:BTCUSDT".into(),
            data,
        };
        AppModel::new(
            config,
            fields,
            vec![row],
            false,
            DEFAULT_WATCH_INTERVAL_SECS,
        )
    }

    fn buffer_text(terminal: &Terminal<TestBackend>) -> String {
        let buf = terminal.backend().buffer();
        let area = buf.area();
        let mut out = String::new();
        for y in 0..area.height {
            for x in 0..area.width {
                out.push_str(buf[(x, y)].symbol());
            }
            out.push('\n');
        }
        out
    }

    #[test]
    fn draw_results_shows_title_and_symbol() {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");
        let model = sample_model();
        terminal.draw(|f| draw(f, &model)).expect("draw");
        let text = buffer_text(&terminal);
        assert!(text.contains("tvscreener-tui"), "{text}");
        assert!(text.contains("BINANCE:BTCUSDT"), "{text}");
        assert!(text.contains("1.25%"), "{text}");
    }

    #[test]
    fn draw_help_lists_keys() {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");
        let mut model = sample_model();
        model.view = ViewMode::Help;
        terminal.draw(|f| draw(f, &model)).expect("draw");
        let text = buffer_text(&terminal);
        assert!(text.contains("refresh"), "{text}");
        assert!(text.contains("watch"), "{text}");
        assert!(text.contains("quit"), "{text}");
    }
}
