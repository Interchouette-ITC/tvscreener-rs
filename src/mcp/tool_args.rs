// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! JSON-schema parameter structs for rmcp `Parameters<T>` tool handlers.

#![allow(missing_docs)]

use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DiscoverFieldsArgs {
    pub search_term: String,
    #[serde(default)]
    pub asset_type: Option<String>,
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct AssetTypeArgs {
    #[serde(default)]
    pub asset_type: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct CustomQueryArgs {
    #[serde(default)]
    pub asset_type: Option<String>,
    #[serde(default)]
    pub fields: Option<String>,
    #[serde(default)]
    pub filters: Option<String>,
    #[serde(default)]
    pub sort_by: Option<String>,
    #[serde(default)]
    pub ascending: Option<bool>,
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct SearchStocksArgs {
    #[serde(default)]
    pub price_range: Option<String>,
    #[serde(default)]
    pub market_cap_billions_range: Option<String>,
    #[serde(default)]
    pub sectors: Option<String>,
    #[serde(default)]
    pub sort_by: Option<String>,
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct SearchCryptoArgs {
    #[serde(default)]
    pub min_volume_millions: Option<f64>,
    #[serde(default)]
    pub min_market_cap_billions: Option<f64>,
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct SearchForexArgs {
    #[serde(default)]
    pub min_volume_millions: Option<f64>,
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct GetTopMoversArgs {
    #[serde(default)]
    pub asset_type: Option<String>,
    #[serde(default)]
    pub direction: Option<String>,
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetPresetArgs {
    pub name: String,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct BuildPayloadArgs {
    #[serde(default)]
    pub asset_type: Option<String>,
    #[serde(default)]
    pub fields: Option<String>,
    #[serde(default)]
    pub filters: Option<String>,
    #[serde(default)]
    pub sort_by: Option<String>,
    #[serde(default)]
    pub indices: Option<String>,
    #[serde(default)]
    pub markets: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchByIndexArgs {
    pub indices: String,
    #[serde(default)]
    pub markets: Option<String>,
    #[serde(default)]
    pub fields: Option<String>,
    #[serde(default)]
    pub sort_by: Option<String>,
    #[serde(default)]
    pub limit: Option<u32>,
}
