// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! Ratatui draw routines for the screener TUI.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, Wrap};
use ratatui::Frame;
use serde_json::Value;

use super::model::{AppModel, BuilderFocus, InputMode, ViewMode, FILTER_OPS};
use super::scan::{payload_pretty, render_codegen};
use super::style::tone_color;
use crate::beautify::{format_cell_for_field, select_table_fields, RowTechMap};
use crate::util::get_columns_to_request;

const LABEL: Style = Style::new().fg(Color::DarkGray);
const ACCENT: Style = Style::new().fg(Color::Cyan);
const MUTED: Style = Style::new().fg(Color::Gray);
const HIGHLIGHT: Style = Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD);
const ACTIVE: Style = Style::new().fg(Color::Green);

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
        ViewMode::Builder => draw_builder(frame, chunks[1], model),
        ViewMode::Payload => draw_payload(frame, chunks[1], model),
        ViewMode::Codegen => draw_codegen(frame, chunks[1], model),
        ViewMode::Help => draw_help(frame, chunks[1]),
    }
    draw_footer(frame, chunks[2], model);
}

fn draw_title(frame: &mut Frame<'_>, area: Rect, model: &AppModel) {
    let preset = model.config.preset.as_deref().unwrap_or("(defaults)");
    let tabs = [
        ViewMode::Results.tab_label(),
        ViewMode::Builder.tab_label(),
        ViewMode::Payload.tab_label(),
        ViewMode::Codegen.tab_label(),
    ];
    let tab_spans: Vec<Span<'_>> = tabs
        .iter()
        .flat_map(|label| {
            let style = if *label == model.view.tab_label() {
                ACCENT.add_modifier(Modifier::BOLD)
            } else {
                MUTED
            };
            [Span::styled(*label, style), Span::raw("  ")]
        })
        .collect();
    let title = Paragraph::new(Line::from(vec![
        Span::styled("tvscreener-tui", ACCENT.add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(preset, MUTED),
        Span::raw("  "),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(Line::from(tab_spans)),
    );
    frame.render_widget(title, area);
}

fn draw_footer(frame: &mut Frame<'_>, area: Rect, model: &AppModel) {
    let status = model.error.as_deref().unwrap_or(model.status.as_str());
    let style = if model.error.is_some() {
        Style::new().fg(Color::Red)
    } else {
        MUTED
    };
    let keys = match model.view {
        ViewMode::Results => {
            "Tab views · r refresh · a watch · c copy rows · h help · ↑↓ scroll"
        }
        ViewMode::Builder => {
            "Tab views · ←→ asset/preset · +/- limit · s search · t/u sort · m/i scope · f filter · Enter scan · j/k focus"
        }
        ViewMode::Payload | ViewMode::Codegen => "Tab views · c copy · ↑↓ scroll · h help",
        ViewMode::Help => "Tab views · h back · q quit",
    };
    let line = Paragraph::new(Line::from(vec![
        Span::styled(status, style),
        Span::raw("  "),
        Span::styled(keys, LABEL),
    ]));
    frame.render_widget(line, area);
}

fn draw_help(frame: &mut Frame<'_>, area: Rect) {
    let lines = vec![
        Line::from(Span::styled("Views", ACCENT.add_modifier(Modifier::BOLD))),
        Line::from("  Tab / 1–4   Results · Builder · Payload · Codegen"),
        Line::from("  h           toggle this help"),
        Line::from(""),
        Line::from(Span::styled("Scan", ACCENT.add_modifier(Modifier::BOLD))),
        Line::from("  r                 refresh (min 10s between scans)"),
        Line::from("  a                 toggle watch (default 30s)"),
        Line::from("  Enter (Builder)   apply config and scan"),
        Line::from(""),
        Line::from(Span::styled("Builder", ACCENT.add_modifier(Modifier::BOLD))),
        Line::from("  ← / →             cycle asset (Asset row) or preset (Preset row)"),
        Line::from("  + / -             row limit"),
        Line::from("  s                 edit name search"),
        Line::from("  t / u             edit sort field / toggle ascending (Sort row)"),
        Line::from("  m / i             edit markets / index (Stock, row focused)"),
        Line::from("  f                 add field filter (pick field, op, value)"),
        Line::from("  d                 remove selected filter"),
        Line::from("  j / k             move focus between rows"),
        Line::from(""),
        Line::from(Span::styled("Copy", ACCENT.add_modifier(Modifier::BOLD))),
        Line::from("  c   Results: rows JSON · Payload: request JSON · Codegen: snippets"),
        Line::from("  Uses OSC 52 (terminal clipboard)."),
        Line::from(""),
        Line::from(Span::styled("Launch", ACCENT.add_modifier(Modifier::BOLD))),
        Line::from("  cargo run --bin tvscreener-tui -- crypto --limit 10"),
        Line::from(
            "  Edit asset, preset, limit, search, sort, markets, index, filters in Builder.",
        ),
    ];
    let body = Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" Help "));
    frame.render_widget(body, area);
}

fn draw_builder(frame: &mut Frame<'_>, area: Rect, model: &AppModel) {
    let mut lines = builder_header_lines(model);
    lines.extend(builder_filter_lines(model));
    lines.push(Line::from(""));
    lines.extend(input_hint_lines(model));

    let body = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((u16::try_from(model.scroll).unwrap_or(0), 0))
        .block(Block::default().borders(Borders::ALL).title(" Builder "));
    frame.render_widget(body, area);
}

fn builder_header_lines(model: &AppModel) -> Vec<Line<'static>> {
    let preset_name = model
        .builder
        .preset_options
        .get(model.builder.preset_index)
        .map_or("(defaults)", String::as_str);
    let search = optional_config_text(model.config.search.as_deref());
    let sort = optional_config_text(model.config.sort_by.as_deref());
    let sort_dir = if model.config.ascending {
        "asc"
    } else {
        "desc"
    };
    let markets = stock_scope_text(model, model.config.markets.as_deref());
    let index = stock_scope_text(model, model.config.index.as_deref());
    vec![
        Line::from(Span::styled(
            "Query builder",
            ACCENT.add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        builder_row(
            model,
            BuilderFocus::Asset,
            "Asset",
            &format!("{}  (← → cycle)", model.config.asset.as_str()),
        ),
        builder_row(
            model,
            BuilderFocus::Preset,
            "Preset",
            &format!("{preset_name}  (← → cycle)"),
        ),
        builder_row(
            model,
            BuilderFocus::Limit,
            "Limit",
            &format!("{} rows  (+ / -)", model.config.limit),
        ),
        builder_row(
            model,
            BuilderFocus::Search,
            "Search",
            &format!("{search}  (s edit)"),
        ),
        builder_row(
            model,
            BuilderFocus::Sort,
            "Sort",
            &format!("{sort} {sort_dir}  (t edit · u toggle)"),
        ),
        builder_row(
            model,
            BuilderFocus::Markets,
            "Markets",
            &format!("{markets}  (m edit)"),
        ),
        builder_row(
            model,
            BuilderFocus::Index,
            "Index",
            &format!("{index}  (i edit)"),
        ),
        Line::from(""),
        builder_row(
            model,
            BuilderFocus::Filters,
            "Filters",
            "(f add · d remove · j/k select)",
        ),
    ]
}

fn builder_filter_lines(model: &AppModel) -> Vec<Line<'static>> {
    if model.config.filters.is_empty() {
        return vec![Line::from(Span::styled("  (no filters)", MUTED))];
    }
    model
        .config
        .filters
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let marker = if i == model.builder.filter_selected {
                ">"
            } else {
                " "
            };
            let style = if i == model.builder.filter_selected {
                HIGHLIGHT
            } else {
                MUTED
            };
            Line::from(Span::styled(
                format!("  {marker} {} {} {}", f.left, f.operation, f.value),
                style,
            ))
        })
        .collect()
}

fn stock_scope_text(model: &AppModel, value: Option<&str>) -> String {
    if model.config.asset != crate::field::Asset::Stock {
        return "(n/a)".into();
    }
    value
        .filter(|s| !s.is_empty())
        .map_or_else(|| "(none)".into(), String::from)
}

fn optional_config_text(value: Option<&str>) -> String {
    value
        .filter(|s| !s.is_empty())
        .map_or_else(|| "(none)".into(), String::from)
}

fn builder_row(model: &AppModel, row: BuilderFocus, label: &str, value: &str) -> Line<'static> {
    let style = if model.builder.focus == row {
        HIGHLIGHT
    } else {
        Style::new()
    };
    Line::from(vec![
        Span::styled(format!("{label:8}"), style),
        Span::raw(" "),
        Span::styled(
            value.to_string(),
            if model.builder.focus == row {
                ACTIVE
            } else {
                MUTED
            },
        ),
    ])
}

fn prompt_input_lines(label: &str, buf: &str) -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(Span::styled(format!("{label}: {buf}_"), ACTIVE)),
        Line::from(Span::styled("Enter save · Esc cancel", LABEL)),
    ]
}

fn input_hint_lines(model: &AppModel) -> Vec<Line<'static>> {
    match model.builder.input_mode {
        InputMode::None => Vec::new(),
        InputMode::Search => prompt_input_lines("Search", &model.builder.input_buf),
        InputMode::Sort => prompt_input_lines("Sort field", &model.builder.input_buf),
        InputMode::Markets => prompt_input_lines("Markets CSV", &model.builder.input_buf),
        InputMode::Index => prompt_input_lines("Index CSV", &model.builder.input_buf),
        InputMode::FilterField => {
            let mut out = vec![
                Line::from(""),
                Line::from(Span::styled(
                    format!("Field search: {}_", model.builder.input_buf),
                    ACTIVE,
                )),
            ];
            for (i, field) in model.builder.field_matches.iter().take(8).enumerate() {
                let pick = if i == model.builder.field_pick {
                    ">"
                } else {
                    " "
                };
                let style = if i == model.builder.field_pick {
                    HIGHLIGHT
                } else {
                    MUTED
                };
                out.push(Line::from(Span::styled(
                    format!("  {pick} {} ({})", field.field_name, field.label),
                    style,
                )));
            }
            out.push(Line::from(Span::styled(
                "j/k pick · Enter next · Esc cancel",
                LABEL,
            )));
            out
        }
        InputMode::FilterOp => {
            let op = FILTER_OPS
                .get(model.builder.op_index)
                .copied()
                .unwrap_or(FILTER_OPS[0]);
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    format!("Operator: {op}  (o cycle · Enter value)"),
                    ACTIVE,
                )),
            ]
        }
        InputMode::FilterValue => vec![
            Line::from(""),
            Line::from(Span::styled(
                format!(
                    "Value for {}: {}_",
                    model.builder.draft_field.as_deref().unwrap_or("?"),
                    model.builder.input_buf
                ),
                ACTIVE,
            )),
            Line::from(Span::styled("Enter add · Esc cancel", LABEL)),
        ],
    }
}

fn draw_payload(frame: &mut Frame<'_>, area: Rect, model: &AppModel) {
    let text = match payload_pretty(&model.config, false) {
        Ok(json) => json,
        Err(err) => format!("payload error: {err}"),
    };
    let body = Paragraph::new(text)
        .wrap(Wrap { trim: false })
        .scroll((u16::try_from(model.scroll).unwrap_or(0), 0))
        .style(MUTED)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Payload (build_payload JSON) "),
        );
    frame.render_widget(body, area);
}

fn draw_codegen(frame: &mut Frame<'_>, area: Rect, model: &AppModel) {
    let text = render_codegen(&model.config);
    let body = Paragraph::new(text)
        .wrap(Wrap { trim: false })
        .scroll((u16::try_from(model.scroll).unwrap_or(0), 0))
        .style(MUTED)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Codegen (Rust + CLI) "),
        );
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::{Asset, FieldDef};
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
            sort_by: None,
            ascending: false,
            filters: Vec::new(),
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
        assert!(text.contains("Builder"), "{text}");
        assert!(text.contains("Payload"), "{text}");
    }

    #[test]
    fn draw_builder_shows_asset_and_sort_rows() {
        let backend = TestBackend::new(80, 28);
        let mut terminal = Terminal::new(backend).expect("terminal");
        let mut model = sample_model();
        model.view = ViewMode::Builder;
        terminal.draw(|f| draw(f, &model)).expect("draw");
        let text = buffer_text(&terminal);
        assert!(text.contains("Asset"), "{text}");
        assert!(text.contains("Sort"), "{text}");
        assert!(text.contains("crypto"), "{text}");
    }

    #[test]
    fn draw_builder_shows_preset() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("terminal");
        let mut model = sample_model();
        model.view = ViewMode::Builder;
        terminal.draw(|f| draw(f, &model)).expect("draw");
        let text = buffer_text(&terminal);
        assert!(text.contains("crypto_price"), "{text}");
    }

    #[test]
    fn draw_payload_renders_filter_key() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("terminal");
        let mut model = sample_model();
        model.view = ViewMode::Payload;
        terminal.draw(|f| draw(f, &model)).expect("draw");
        let text = buffer_text(&terminal);
        assert!(text.contains("filter"), "{text}");
    }
}
