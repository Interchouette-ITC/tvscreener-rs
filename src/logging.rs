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
/// Requires the `cli` Cargo feature (pulled in by `mcp`, `tui`, and `apps`).
///
/// # Panics
///
/// Panics if a global subscriber was already set (call once at process start).
#[cfg(feature = "cli")]
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

#[cfg(test)]
mod tests {
    use super::env_debug_enabled;

    #[test]
    fn env_debug_truthy_and_falsey() {
        let prev = std::env::var_os("TVSCREENER_DEBUG");
        // SAFETY: test-only env mutation; this suite does not run concurrent env readers.
        unsafe {
            std::env::remove_var("TVSCREENER_DEBUG");
        }
        assert!(!env_debug_enabled());

        for truthy in ["1", "true", "YES", "on"] {
            unsafe {
                std::env::set_var("TVSCREENER_DEBUG", truthy);
            }
            assert!(env_debug_enabled(), "expected truthy for {truthy}");
        }
        for falsey in ["0", "false", "off", "no"] {
            unsafe {
                std::env::set_var("TVSCREENER_DEBUG", falsey);
            }
            assert!(!env_debug_enabled(), "expected falsey for {falsey}");
        }

        unsafe {
            match prev {
                Some(v) => std::env::set_var("TVSCREENER_DEBUG", v),
                None => std::env::remove_var("TVSCREENER_DEBUG"),
            }
        }
    }
}
