// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! JSON-schema parameter structs for rmcp `Parameters<T>` tool handlers.

use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct DiscoverFieldsArgs {
    pub(crate) search_term: String,
    #[serde(default)]
    pub(crate) asset_type: Option<String>,
    #[serde(default)]
    pub(crate) limit: Option<u32>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub(crate) struct AssetTypeArgs {
    #[serde(default)]
    pub(crate) asset_type: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub(crate) struct CustomQueryArgs {
    #[serde(default)]
    pub(crate) asset_type: Option<String>,
    #[serde(default)]
    pub(crate) fields: Option<String>,
    #[serde(default)]
    pub(crate) filters: Option<String>,
    #[serde(default)]
    pub(crate) sort_by: Option<String>,
    #[serde(default)]
    pub(crate) ascending: Option<bool>,
    #[serde(default)]
    pub(crate) limit: Option<u32>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub(crate) struct SearchStocksArgs {
    #[serde(default)]
    pub(crate) price_range: Option<String>,
    #[serde(default)]
    pub(crate) market_cap_billions_range: Option<String>,
    #[serde(default)]
    pub(crate) sectors: Option<String>,
    #[serde(default)]
    pub(crate) sort_by: Option<String>,
    #[serde(default)]
    pub(crate) limit: Option<u32>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub(crate) struct SearchCryptoArgs {
    #[serde(default)]
    pub(crate) min_volume_millions: Option<f64>,
    #[serde(default)]
    pub(crate) min_market_cap_billions: Option<f64>,
    #[serde(default)]
    pub(crate) limit: Option<u32>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub(crate) struct SearchForexArgs {
    #[serde(default)]
    pub(crate) min_volume_millions: Option<f64>,
    #[serde(default)]
    pub(crate) limit: Option<u32>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub(crate) struct GetTopMoversArgs {
    #[serde(default)]
    pub(crate) asset_type: Option<String>,
    #[serde(default)]
    pub(crate) direction: Option<String>,
    #[serde(default)]
    pub(crate) limit: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct GetPresetArgs {
    pub(crate) name: String,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub(crate) struct BuildPayloadArgs {
    #[serde(default)]
    pub(crate) asset_type: Option<String>,
    #[serde(default)]
    pub(crate) fields: Option<String>,
    #[serde(default)]
    pub(crate) filters: Option<String>,
    #[serde(default)]
    pub(crate) sort_by: Option<String>,
    #[serde(default)]
    pub(crate) indices: Option<String>,
    #[serde(default)]
    pub(crate) markets: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SearchByIndexArgs {
    pub(crate) indices: String,
    #[serde(default)]
    pub(crate) markets: Option<String>,
    #[serde(default)]
    pub(crate) fields: Option<String>,
    #[serde(default)]
    pub(crate) sort_by: Option<String>,
    #[serde(default)]
    pub(crate) limit: Option<u32>,
}
