// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! Regenerate `data/fields.json` and `src/field/generated/` from a Python tvscreener clone.

use anyhow::{bail, Context, Result};
use regex::Regex;
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const ASSETS: &[(&str, &str)] = &[
    ("crypto", "tvscreener/field/crypto.py"),
    ("stock", "tvscreener/field/stock.py"),
    ("forex", "tvscreener/field/forex.py"),
    ("bond", "tvscreener/field/bond.py"),
    ("futures", "tvscreener/field/futures.py"),
    ("coin", "tvscreener/field/coin.py"),
];

const GEN_DIR_REL: &str = "src/field/generated";

fn field_line_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r#"(?m)^\s+([A-Z][A-Z0-9_]*)\s*=\s*(?:'([^']*)'|"([^"]*)")\s*,\s*(?:'([^']*)'|"([^"]*)")(?:\s*,\s*(?:'([^']*)'|"([^"]*)"|None))?(?:\s*,\s*(True|False))?(?:\s*,\s*(True|False))?"#,
        )
        .expect("field line regex")
    })
}

fn default_list_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?s)DEFAULT_([A-Z]+)_FIELDS\s*=\s*\[(.*?)\]").expect("default list regex")
    })
}

fn index_line_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?m)^\s+([A-Z][A-Z0-9_]*)\s*=\s*\("([^"]+)",\s*"([^"]+)"\)"#)
            .expect("index line regex")
    })
}

fn string_member_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?m)^\s+([A-Z][A-Z0-9_]*)\s*=\s*(?:'([^']*)'|"([^"]*)")\s*(?:#.*)?$"#)
            .expect("string member regex")
    })
}

fn rating_member_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r#"(?m)^\s+([A-Z][A-Z0-9_]*)\s*=\s*([^,]+)\s*,\s*([^,]+)\s*,\s*(?:'([^']*)'|"([^"]*)")"#,
        )
        .expect("rating member regex")
    })
}

fn camel_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\b([A-Z][a-z]+[A-Z][A-Za-z]*)\b").expect("camel regex"))
}

fn class_member_re(class_name: &str) -> Regex {
    Regex::new(&format!(r"{class_name}\.([A-Z][A-Z0-9_]*)")).expect("class member regex")
}

fn load_existing(out: &Path) -> Result<Value> {
    if !out.is_file() {
        return Ok(json!({}));
    }
    serde_json::from_str(
        &fs::read_to_string(out).with_context(|| format!("read {}", out.display()))?,
    )
    .with_context(|| format!("parse {}", out.display()))
}

fn fill_asset_catalog(
    python_root: &Path,
    existing: &Value,
    catalog: &mut Map<String, Value>,
    defaults: &mut Map<String, Value>,
) -> Result<()> {
    for &(asset, rel) in ASSETS {
        let path = python_root.join(rel);
        let text = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        let fields = parse_fields(&text);
        let field_count = fields.len();
        let mut default_names = parse_defaults(&text, asset);
        default_names.retain(|n| fields.contains_key(n));
        if default_names.is_empty() {
            if let Some(prev) = existing
                .pointer(&format!("/defaults/{asset}"))
                .and_then(Value::as_array)
            {
                default_names = prev
                    .iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .filter(|n| fields.contains_key(n))
                    .collect();
            }
            if default_names.is_empty() {
                default_names = fields.keys().take(50).cloned().collect();
            }
        }
        let default_count = default_names.len();
        catalog.insert(asset.to_string(), Value::Object(fields_to_json(fields)));
        defaults.insert(
            asset.to_string(),
            Value::Array(default_names.into_iter().map(Value::String).collect()),
        );
        println!("{asset}: catalog={field_count} defaults={default_count}");
    }
    Ok(())
}

struct ParsedEnums {
    index_symbols: Vec<Value>,
    markets: Vec<Value>,
    sectors: Vec<Value>,
    countries: Vec<Value>,
    industries: Vec<Value>,
    exchanges: Vec<Value>,
    ratings: Vec<Value>,
}

fn parse_init_enums(init_text: &str) -> ParsedEnums {
    let index_symbols = parse_index_symbols(init_text);
    let markets = parse_string_enum(init_text, "Market");
    let sectors = parse_string_enum(init_text, "Sector");
    let countries = parse_string_enum(init_text, "Country");
    let industries = parse_string_enum(init_text, "Industry");
    let exchanges = parse_string_enum(init_text, "Exchange");
    let ratings = parse_ratings(init_text);
    println!("index_symbols: {}", index_symbols.len());
    println!("markets: {}", markets.len());
    println!("sectors: {}", sectors.len());
    println!("countries: {}", countries.len());
    println!("industries: {}", industries.len());
    println!("exchanges: {}", exchanges.len());
    println!("ratings: {}", ratings.len());
    ParsedEnums {
        index_symbols,
        markets,
        sectors,
        countries,
        industries,
        exchanges,
        ratings,
    }
}

fn build_out_obj(
    existing: &Value,
    catalog: Map<String, Value>,
    defaults: Map<String, Value>,
    enums: &ParsedEnums,
) -> Map<String, Value> {
    let mut out_obj = Map::new();
    out_obj.insert("defaults".into(), Value::Object(defaults));
    out_obj.insert("catalog".into(), Value::Object(catalog));
    out_obj.insert(
        "presets".into(),
        existing
            .get("presets")
            .cloned()
            .unwrap_or_else(|| json!({})),
    );
    let markets = if enums.markets.is_empty() {
        existing
            .get("markets")
            .cloned()
            .unwrap_or_else(|| json!([]))
    } else {
        Value::Array(enums.markets.clone())
    };
    out_obj.insert("markets".into(), markets);
    out_obj.insert(
        "types".into(),
        existing.get("types").cloned().unwrap_or_else(|| json!([])),
    );
    out_obj.insert(
        "symbol_types".into(),
        existing
            .get("symbol_types")
            .cloned()
            .unwrap_or_else(|| json!([])),
    );
    out_obj.insert(
        "index_symbols".into(),
        Value::Array(enums.index_symbols.clone()),
    );
    out_obj.insert("sectors".into(), Value::Array(enums.sectors.clone()));
    out_obj.insert("countries".into(), Value::Array(enums.countries.clone()));
    out_obj.insert("industries".into(), Value::Array(enums.industries.clone()));
    out_obj.insert("exchanges".into(), Value::Array(enums.exchanges.clone()));
    out_obj.insert("ratings".into(), Value::Array(enums.ratings.clone()));
    out_obj
}

fn write_generated_modules(crate_root: &Path, enums: &ParsedEnums) -> Result<()> {
    let gen_dir = crate_root.join(GEN_DIR_REL);
    fs::create_dir_all(&gen_dir).with_context(|| format!("mkdir {}", gen_dir.display()))?;
    write_index_symbol_rs(&enums.index_symbols, &gen_dir.join("index_symbol.rs"))?;
    write_named_str_rs(&enums.sectors, &gen_dir.join("sector.rs"), "sector")?;
    write_named_str_rs(&enums.countries, &gen_dir.join("country.rs"), "country")?;
    write_named_str_rs(&enums.industries, &gen_dir.join("industry.rs"), "industry")?;
    write_named_str_rs(&enums.exchanges, &gen_dir.join("exchange.rs"), "exchange")?;
    write_rating_rs(&enums.ratings, &gen_dir.join("rating.rs"))?;
    println!("wrote generated modules under {}", gen_dir.display());
    Ok(())
}

/// Run catalog regeneration.
pub fn run(python_root: &Path, crate_root: &Path, out: &Path) -> Result<()> {
    let field_dir = python_root.join("tvscreener").join("field");
    if !field_dir.is_dir() {
        bail!(
            "{} does not look like a tvscreener clone (expected tvscreener/field/)",
            python_root.display()
        );
    }

    let existing = load_existing(out)?;
    let mut catalog = Map::new();
    let mut defaults = Map::new();
    fill_asset_catalog(python_root, &existing, &mut catalog, &mut defaults)?;

    let init_path = field_dir.join("__init__.py");
    let init_text =
        fs::read_to_string(&init_path).with_context(|| format!("read {}", init_path.display()))?;
    let enums = parse_init_enums(&init_text);
    let out_obj = build_out_obj(&existing, catalog, defaults, &enums);

    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    let pretty = serde_json::to_string_pretty(&Value::Object(out_obj))?;
    fs::write(out, format!("{pretty}\n")).with_context(|| format!("write {}", out.display()))?;
    println!("wrote {}", out.display());

    write_generated_modules(crate_root, &enums)
}

fn fields_to_json(fields: BTreeMap<String, FieldEntry>) -> Map<String, Value> {
    let mut map = Map::new();
    for (name, entry) in fields {
        let mut obj = Map::new();
        obj.insert("label".into(), json!(entry.label));
        obj.insert("field_name".into(), json!(entry.field_name));
        obj.insert("interval".into(), json!(entry.interval));
        obj.insert("historical".into(), json!(entry.historical));
        if let Some(fmt) = entry.format {
            obj.insert("format".into(), json!(fmt));
        }
        map.insert(name, Value::Object(obj));
    }
    map
}

#[derive(Clone)]
struct FieldEntry {
    label: String,
    field_name: String,
    format: Option<String>,
    interval: bool,
    historical: bool,
}

fn parse_fields(text: &str) -> BTreeMap<String, FieldEntry> {
    let mut out = BTreeMap::new();
    for caps in field_line_re().captures_iter(text) {
        let name = caps[1].to_string();
        let label = caps
            .get(2)
            .or_else(|| caps.get(3))
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();
        let field_name = caps
            .get(4)
            .or_else(|| caps.get(5))
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();
        let fmt = caps
            .get(6)
            .or_else(|| caps.get(7))
            .map(|m| m.as_str().to_string())
            .filter(|s| s != "None");
        let interval = caps.get(8).is_some_and(|m| m.as_str() == "True");
        let historical = caps.get(9).is_some_and(|m| m.as_str() == "True");
        out.insert(
            name,
            FieldEntry {
                label,
                field_name,
                format: fmt,
                interval,
                historical,
            },
        );
    }
    out
}

fn parse_defaults(text: &str, asset: &str) -> Vec<String> {
    let class_name = match asset {
        "crypto" => "CryptoField",
        "stock" => "StockField",
        "forex" => "ForexField",
        "bond" => "BondField",
        "futures" => "FuturesField",
        "coin" => "CoinField",
        _ => return Vec::new(),
    };
    let key = asset.to_ascii_uppercase();
    let Some(caps) = default_list_re()
        .captures_iter(text)
        .find(|c| c.get(1).map(|m| m.as_str()) == Some(key.as_str()))
    else {
        return Vec::new();
    };
    let body = caps.get(2).map_or("", |m| m.as_str());
    class_member_re(class_name)
        .captures_iter(body)
        .filter_map(|c| c.get(1).map(|m| m.as_str().to_string()))
        .collect()
}

fn enum_class_body(init_text: &str, class_name: &str) -> String {
    let marker = format!("class {class_name}");
    let Some(start) = init_text.find(&marker) else {
        return String::new();
    };
    let after = &init_text[start..];
    let rest = after.split_once('\n').map_or("", |(_, r)| r);
    rest.find("\nclass ")
        .map_or_else(|| rest.to_string(), |idx| rest[..idx].to_string())
}

fn parse_string_enum(init_text: &str, class_name: &str) -> Vec<Value> {
    let body = enum_class_body(init_text, class_name);
    string_member_re()
        .captures_iter(&body)
        .map(|caps| {
            let name = &caps[1];
            let value = caps
                .get(2)
                .or_else(|| caps.get(3))
                .map_or("", |m| m.as_str());
            json!({ "const": name, "value": value })
        })
        .collect()
}

fn parse_index_symbols(init_text: &str) -> Vec<Value> {
    let body = enum_class_body(init_text, "IndexSymbol");
    index_line_re()
        .captures_iter(&body)
        .map(|caps| {
            json!({
                "const": &caps[1],
                "value": &caps[2],
                "label": &caps[3],
            })
        })
        .collect()
}

fn parse_floatish(raw: &str) -> Option<f64> {
    let s = raw.trim();
    if s.to_ascii_lowercase().contains("nan") {
        return None;
    }
    s.parse().ok()
}

fn parse_ratings(init_text: &str) -> Vec<Value> {
    let body = enum_class_body(init_text, "Rating");
    rating_member_re()
        .captures_iter(&body)
        .map(|caps| {
            let label = caps
                .get(4)
                .or_else(|| caps.get(5))
                .map_or("", |m| m.as_str());
            json!({
                "const": &caps[1],
                "min": parse_floatish(&caps[2]),
                "max": parse_floatish(&caps[3]),
                "label": label,
            })
        })
        .collect()
}

fn rust_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn doc_label(label: &str) -> String {
    let cleaned = label.replace("*/", "* /");
    camel_re().replace_all(&cleaned, "`$1`").into_owned()
}

fn write_header() -> Vec<String> {
    vec![
        "// Copyright 2026 tvscreener-rs contributors".into(),
        "// SPDX-License-Identifier: Apache-2.0".into(),
        String::new(),
        "// Generated by `tvscreener regen-fields`. Do not edit by hand.".into(),
        String::new(),
    ]
}

fn write_named_str_rs(entries: &[Value], out_path: &Path, kind: &str) -> Result<()> {
    let mut lines = write_header();
    let mut names = Vec::new();
    for entry in entries {
        let name = entry["const"].as_str().unwrap_or_default();
        let value = entry["value"].as_str().unwrap_or_default();
        let label = entry.get("label").and_then(Value::as_str).unwrap_or(value);
        names.push(name.to_string());
        lines.push(format!("/// {} (`{value}`).", doc_label(label)));
        lines.push(format!(
            "pub const {name}: &str = \"{}\";",
            rust_escape(value)
        ));
    }
    lines.push(String::new());
    lines.push(format!("/// All known {kind} constant names."));
    lines.push("pub const ALL_CONST_NAMES: &[&str] = &[".into());
    for name in &names {
        lines.push(format!("    \"{name}\","));
    }
    lines.push("];".into());
    lines.push(String::new());
    fs::write(out_path, lines.join("\n") + "\n")
        .with_context(|| format!("write {}", out_path.display()))
}

fn write_index_symbol_rs(index_symbols: &[Value], out_path: &Path) -> Result<()> {
    let mut lines = write_header();
    let mut names = Vec::new();
    for idx in index_symbols {
        let name = idx["const"].as_str().unwrap_or_default();
        let value = idx["value"].as_str().unwrap_or_default();
        let label = idx.get("label").and_then(Value::as_str).unwrap_or(name);
        names.push(name.to_string());
        lines.push(format!("/// {} (`{value}`).", doc_label(label)));
        lines.push(format!(
            "pub const {name}: &str = \"{}\";",
            rust_escape(value)
        ));
    }
    lines.push(String::new());
    lines.push("/// All known index constant names.".into());
    lines.push("pub const ALL_CONST_NAMES: &[&str] = &[".into());
    for name in &names {
        lines.push(format!("    \"{name}\","));
    }
    lines.push("];".into());
    lines.push(String::new());
    fs::write(out_path, lines.join("\n") + "\n")
        .with_context(|| format!("write {}", out_path.display()))
}

fn rust_f64_literal(v: Option<f64>) -> String {
    v.map_or_else(
        || "f64::NAN".into(),
        |n| {
            let s = format!("{n}");
            if s.contains('.') || s.contains('e') || s.contains('E') {
                s
            } else {
                format!("{s}.0")
            }
        },
    )
}

fn write_rating_rs(ratings: &[Value], out_path: &Path) -> Result<()> {
    let mut lines = write_header();
    let mut names = Vec::new();
    for entry in ratings {
        let name = entry["const"].as_str().unwrap_or_default();
        let label = entry["label"].as_str().unwrap_or_default();
        let lo = entry.get("min").and_then(Value::as_f64);
        // JSON null becomes None via as_f64; also handle explicit null.
        let lo = if entry.get("min").is_some_and(Value::is_null) {
            None
        } else {
            lo
        };
        let hi = if entry.get("max").is_some_and(Value::is_null) {
            None
        } else {
            entry.get("max").and_then(Value::as_f64)
        };
        names.push(name.to_string());
        lines.push(format!("/// {}.", doc_label(label)));
        lines.push(format!("pub const {name}: RatingBand = RatingBand {{"));
        lines.push(format!("    const_name: \"{name}\","));
        lines.push(format!("    min: {},", rust_f64_literal(lo)));
        lines.push(format!("    max: {},", rust_f64_literal(hi)));
        lines.push(format!("    label: \"{}\",", rust_escape(label)));
        lines.push("};".into());
    }
    lines.push(String::new());
    lines.push("/// All rating bands in declaration order.".into());
    lines.push("pub const ALL: &[RatingBand] = &[".into());
    for name in &names {
        lines.push(format!("    {name},"));
    }
    lines.push("];".into());
    lines.push(String::new());
    lines.push("/// All known rating constant names.".into());
    lines.push("pub const ALL_CONST_NAMES: &[&str] = &[".into());
    for name in &names {
        lines.push(format!("    \"{name}\","));
    }
    lines.push("];".into());
    lines.push(String::new());
    fs::write(out_path, lines.join("\n") + "\n")
        .with_context(|| format!("write {}", out_path.display()))
}

/// Resolve default output path under the crate root.
pub fn default_out(crate_root: &Path) -> PathBuf {
    crate_root.join("data").join("fields.json")
}
