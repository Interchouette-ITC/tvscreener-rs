// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! Ratatui screener TUI (feature `tui`).
//!
//! Scaffold: Results table + Help. Builder / Payload panes land in a follow-up.

mod clipboard;
mod draw;
mod model;
mod style;
mod terminal;

pub use clipboard::copy_via_osc52;
pub use draw::draw;
pub use model::{
    AppModel, ScanConfig, ViewMode, DEFAULT_WATCH_INTERVAL_SECS, MIN_TUI_REFRESH_SECS,
};
pub use terminal::{
    hard_reset_tty, inside_gnu_screen, install_panic_hook, install_signal_handlers, is_quit_key,
    TerminalGuard, STOP,
};
