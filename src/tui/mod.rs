// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! Ratatui screener TUI (feature `tui`).

mod clipboard;
mod draw;
mod model;
mod scan;
mod style;
mod terminal;

pub use clipboard::copy_via_osc52;
pub use draw::draw;
pub use model::{
    AppModel, BuilderFocus, BuilderUi, InputMode, ScanConfig, ViewMode,
    DEFAULT_WATCH_INTERVAL_SECS, FILTER_OPS, MIN_TUI_REFRESH_SECS,
};
pub use scan::{build_payload, configure_screener, payload_pretty, render_codegen};
pub use terminal::{
    hard_reset_tty, inside_gnu_screen, install_panic_hook, install_signal_handlers, is_quit_key,
    TerminalGuard, STOP,
};
