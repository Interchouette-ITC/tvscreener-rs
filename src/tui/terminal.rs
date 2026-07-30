// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! Terminal raw-mode / alternate-screen lifecycle.

use std::env;
use std::io::{self, stdout, Write};
use std::sync::atomic::{AtomicBool, Ordering};

use crossterm::cursor;
use crossterm::event::{KeyCode, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;

/// Set by SIGINT/SIGTERM so the loop can restore the tty before exit.
pub static STOP: AtomicBool = AtomicBool::new(false);

/// Restores the terminal on drop (normal exit, `?`, or unwind after panic hook).
pub struct TerminalGuard {
    use_alt: bool,
}

impl TerminalGuard {
    /// Enables raw mode and optionally the alternate screen.
    ///
    /// # Errors
    ///
    /// Returns an I/O error when the tty cannot enter raw mode or alt screen.
    pub fn enter(use_alt: bool) -> io::Result<Self> {
        enable_raw_mode()?;
        let mut out = stdout();
        if use_alt {
            out.execute(EnterAlternateScreen)?;
        }
        hard_reset_tty();
        out.execute(cursor::Hide)?;
        out.flush()?;
        Ok(Self { use_alt })
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        restore_terminal(self.use_alt);
    }
}

/// GNU screen sets `STY`. Its default config often ignores the xterm alt buffer.
#[must_use]
pub fn inside_gnu_screen() -> bool {
    env::var_os("STY").is_some()
}

/// Returns true for quit keys (`q`, Esc, Ctrl-C).
#[must_use]
pub const fn is_quit_key(code: KeyCode, modifiers: KeyModifiers) -> bool {
    matches!(code, KeyCode::Char('q') | KeyCode::Esc)
        || (matches!(code, KeyCode::Char('c')) && modifiers.contains(KeyModifiers::CONTROL))
}

/// Installs Ctrl-C / SIGTERM handler that sets [`STOP`].
pub fn install_signal_handlers() {
    let _ = ctrlc::set_handler(|| {
        STOP.store(true, Ordering::SeqCst);
    });
}

/// Restores the tty on panic before the default panic hook runs.
pub fn install_panic_hook() {
    let use_alt = !inside_gnu_screen();
    let prior = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_terminal(use_alt);
        prior(info);
    }));
}

/// Reset SGR, cursor, and clear both alt and main tty buffers.
pub fn hard_reset_tty() {
    let mut out = stdout();
    let _ = write!(
        out,
        "\x1b[0m\x1b[?25h\x1b[?1049l\x1b[?47l\x1b[2J\x1b[3J\x1b[H"
    );
    let _ = out.execute(Clear(ClearType::All));
    let _ = out.execute(Clear(ClearType::Purge));
    let _ = out.execute(cursor::Show);
    let _ = out.flush();
}

fn restore_terminal(use_alt: bool) {
    let _ = disable_raw_mode();
    let mut out = stdout();
    if use_alt {
        let _ = out.execute(LeaveAlternateScreen);
    }
    let _ = out.flush();
    hard_reset_tty();
}
