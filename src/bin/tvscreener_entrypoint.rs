// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! Docker entrypoint: MCP HTTP alone, TUI±MCP, interactive CLI±MCP, or one-shot CLI.
//!
//! Replaces `docker/entrypoint.sh` so the runtime image can be distroless (no `/bin/sh`).

use std::env;
use std::io::{stdin, stdout, IsTerminal};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const BIN_DIR: &str = "/usr/local/bin";
const DEFAULT_MCP_ADDR: &str = "0.0.0.0:6790";

fn main() {
    let mcp_addr = env::var("TVSCREENER_MCP_ADDR").unwrap_or_else(|_| DEFAULT_MCP_ADDR.to_string());
    let args: Vec<String> = env::args().skip(1).collect();

    if !args.is_empty() {
        if is_long_lived(&args) {
            start_mcp_http(&mcp_addr);
        }
        exec_command(&args);
    }

    // No args: detached / no TTY → MCP HTTP only.
    if !has_tty() {
        exec_command(&[
            "tvscreener-mcp".to_string(),
            "--http".to_string(),
            "--listen".to_string(),
            mcp_addr,
        ]);
    }

    // Interactive TTY → TUI (+ MCP unless disabled).
    start_mcp_http(&mcp_addr);
    exec_command(&[
        "tvscreener-tui".to_string(),
        "crypto".to_string(),
        "--limit".to_string(),
        "25".to_string(),
    ]);
}

fn mcp_enabled() -> bool {
    env::var("ENABLE_MCP").map_or(true, |v| {
        !matches!(
            v.as_str(),
            "0" | "false" | "FALSE" | "no" | "NO" | "off" | "OFF"
        )
    })
}

fn start_mcp_http(addr: &str) {
    if !mcp_enabled() {
        return;
    }
    let bin = resolve_bin("tvscreener-mcp");
    let child = Command::new(&bin)
        .args(["--http", "--listen", addr])
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn();
    if let Err(err) = child {
        eprintln!("tvscreener-entrypoint: failed to start MCP: {err}");
        std::process::exit(1);
    }
}

fn has_tty() -> bool {
    stdin().is_terminal() || stdout().is_terminal()
}

/// True when the foreground should keep the container alive (TUI / interactive CLI).
fn is_long_lived(args: &[String]) -> bool {
    let Some(cmd) = args.first() else {
        return true;
    };
    let name = command_basename(cmd);
    match name.as_str() {
        "tvscreener-tui" => true,
        "tvscreener" => {
            let mut rest = &args[1..];
            while let Some(a) = rest.first() {
                match a.as_str() {
                    "--debug" => rest = &rest[1..],
                    "--help" | "-h" => return false,
                    s if s.starts_with('-') => rest = &rest[1..],
                    _ => break,
                }
            }
            // No subcommand → interactive CLI.
            rest.is_empty()
        }
        _ => false,
    }
}

fn command_basename(cmd: &str) -> String {
    Path::new(cmd)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(cmd)
        .to_string()
}

fn resolve_bin(name_or_path: &str) -> PathBuf {
    let path = Path::new(name_or_path);
    if path.is_absolute() || name_or_path.contains('/') {
        return path.to_path_buf();
    }
    Path::new(BIN_DIR).join(name_or_path)
}

fn exec_command(args: &[String]) -> ! {
    let Some((prog, rest)) = args.split_first() else {
        eprintln!("tvscreener-entrypoint: empty command");
        std::process::exit(1);
    };
    let bin = resolve_bin(prog);
    let err = Command::new(&bin).args(rest).exec();
    eprintln!("tvscreener-entrypoint: exec {}: {err}", bin.display());
    std::process::exit(1);
}

#[cfg(test)]
mod tests {
    use super::{command_basename, is_long_lived};

    fn args(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn long_lived_tui() {
        assert!(is_long_lived(&args(&["tvscreener-tui"])));
        assert!(is_long_lived(&args(&[
            "/usr/local/bin/tvscreener-tui",
            "crypto"
        ])));
    }

    #[test]
    fn long_lived_interactive_cli() {
        assert!(is_long_lived(&args(&["tvscreener"])));
        assert!(is_long_lived(&args(&["tvscreener", "--debug"])));
        assert!(is_long_lived(&args(&["/usr/local/bin/tvscreener"])));
    }

    #[test]
    fn not_long_lived_oneshot_cli() {
        assert!(!is_long_lived(&args(&["tvscreener", "--help"])));
        assert!(!is_long_lived(&args(&["tvscreener", "-h"])));
        assert!(!is_long_lived(&args(&["tvscreener", "scan", "crypto"])));
        assert!(!is_long_lived(&args(&[
            "tvscreener",
            "--debug",
            "payload",
            "crypto"
        ])));
    }

    #[test]
    fn not_long_lived_other() {
        assert!(!is_long_lived(&args(&["tvscreener-mcp", "--http"])));
        assert!(!is_long_lived(&args(&["true"])));
    }

    #[test]
    fn basename_strips_path() {
        assert_eq!(
            command_basename("/usr/local/bin/tvscreener-tui"),
            "tvscreener-tui"
        );
        assert_eq!(command_basename("tvscreener"), "tvscreener");
    }
}
