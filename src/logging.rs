// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! Process logging helpers (`tracing` + stderr subscriber install).

/// Returns true when `TVSCREENER_DEBUG` is set to a truthy value (`1`, `true`, `yes`, `on`).
#[must_use]
pub fn env_debug_enabled() -> bool {
    std::env::var("TVSCREENER_DEBUG").is_ok_and(|v| {
        matches!(
            v.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

/// Installs a stderr [`tracing`] subscriber.
///
/// Filter resolution order:
/// 1. `RUST_LOG` (standard [`tracing_subscriber::EnvFilter`])
/// 2. else if [`env_debug_enabled`], `tvscreener=debug,info`
/// 3. else `info`
///
/// Logs go to **stderr** so CLI/MCP stdout stays for data / protocol.
///
/// # Panics
///
/// Panics if a global subscriber was already set (call once at process start).
pub fn init_logging() {
    use tracing_subscriber::EnvFilter;

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        if env_debug_enabled() {
            EnvFilter::new("tvscreener=debug,info")
        } else {
            EnvFilter::new("info")
        }
    });

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_target(true)
        .init();
}
