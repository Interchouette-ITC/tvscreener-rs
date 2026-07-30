// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! Scan result printers for the `tvscreener` CLI.

use std::io::{self, IsTerminal};

use anyhow::Result;
use clap::ValueEnum;
use tvscreener::{
    format_row, format_rows_table, FieldDef, ScreenerRow, TableFormatOptions,
    DEFAULT_TABLE_MAX_COLUMNS,
};

/// How `scan` prints rows.
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum ScanFormat {
    /// Aligned columns with field-aware beautify (default).
    Table,
    /// Legacy one-line `format_row` output.
    Row,
    /// Pretty-printed JSON array of [`ScreenerRow`].
    Json,
}

/// When to emit ANSI colors for table cells.
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum ColorWhen {
    /// Color when stdout is a TTY.
    Auto,
    /// Always color.
    Always,
    /// Never color.
    Never,
}

impl ColorWhen {
    /// Resolves whether ANSI color should be applied.
    #[must_use]
    pub fn enabled(self) -> bool {
        match self {
            Self::Always => true,
            Self::Never => false,
            Self::Auto => io::stdout().is_terminal(),
        }
    }
}

/// Prints scan rows according to format / color flags.
///
/// # Errors
///
/// Returns an error when JSON serialization fails.
pub fn print_scan_rows(
    rows: &[ScreenerRow],
    fields: &[FieldDef],
    format: ScanFormat,
    color: ColorWhen,
    json_flag: bool,
) -> Result<()> {
    let format = if json_flag { ScanFormat::Json } else { format };
    match format {
        ScanFormat::Json => {
            println!("{}", serde_json::to_string_pretty(rows)?);
        }
        ScanFormat::Row => {
            if rows.is_empty() {
                println!("(no rows)");
            } else {
                for row in rows {
                    println!("{}", format_row(row, None));
                }
            }
        }
        ScanFormat::Table => {
            let text = format_rows_table(
                rows,
                fields,
                &TableFormatOptions {
                    max_columns: DEFAULT_TABLE_MAX_COLUMNS,
                    color: color.enabled(),
                },
            );
            print!("{text}");
        }
    }
    Ok(())
}
