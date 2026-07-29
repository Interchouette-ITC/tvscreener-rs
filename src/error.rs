// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! Error types for HTTP, JSON, and request construction failures.

use thiserror::Error;

/// Crate-wide result alias.
pub type Result<T> = std::result::Result<T, TvscreenerError>;

/// Errors produced by payload building, HTTP, or response parsing.
///
/// Wraps network / JSON
/// failures from `reqwest` / `serde_json`.
#[derive(Debug, Error)]
pub enum TvscreenerError {
    /// `TradingView` (or proxy) returned a non-success HTTP status.
    #[error("HTTP {status}: {body}")]
    HttpStatus {
        /// Response status code.
        status: u16,
        /// Response body (truncated when longer than 4 KiB).
        body: String,
    },

    /// Request timed out.
    #[error("request timed out")]
    Timeout,

    /// Underlying network / transport failure.
    #[error("network error: {0}")]
    Network(String),

    /// Response body was not valid JSON or did not match the expected shape.
    #[error("JSON error: {0}")]
    Json(String),

    /// Caller built an invalid filter, range, or column set.
    #[error("invalid request: {0}")]
    InvalidRequest(String),

    /// Filter `left` is not in the allowed field set for this screener.
    #[error("invalid field name: {0}")]
    InvalidFieldName(String),
}

impl From<reqwest::Error> for TvscreenerError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            Self::Timeout
        } else {
            Self::Network(err.to_string())
        }
    }
}

impl From<serde_json::Error> for TvscreenerError {
    fn from(err: serde_json::Error) -> Self {
        Self::Json(err.to_string())
    }
}
