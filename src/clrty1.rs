//! CLRTY-1 L1 RPC client (duplicated into Compiler-Bridge).
//! Multi-endpoint failover: primary RPC → fallback RPC → api/rpc → api chain affirm → exchange/health.

use serde_json::{json, Value};

pub const CLRTY1_CHAIN_ID: &str = "clrty-1";
pub const CLRTY1_NUMERIC_CHAIN_ID: u64 = 1202;
pub const CLRTY1_DENOM: &str = "uclrty";

pub const DEFAULT_RPC: &str = "https://rpc.clarity-fintech.com";
pub const DEFAULT_API_BASE: &str = "https://api.clarity-fintech.com";
pub const DEFAULT_EXCHANGE_HEALTH: &str = "https://exchange.clarity-fintech.com/health";

pub struct Clrty1Config {
    pub rpc_url: String,
    pub rpc_fallback_url: Option<String>,
    pub api_base: String,
    pub exchange_health_url: String,
    pub chain_id: String,
    pub numeric_chain_id: u64,
}

impl Clrty1Config {
    pub fn from_env() -> Self {
        Self {
            rpc_url: std::env::var("CLRTY_L1_RPC")
                .or_else(|_| std::env::var("CLRTY_L1_RPC_URL"))
                .unwrap_or_else(|_| DEFAULT_RPC.into()),
            rpc_fallback_url: std::env::var("CLRTY_L1_RPC_FALLBACK").ok(),
            api_base: std::env::var("CLRTY_API_BASE")
                .unwrap_or_else(|_| DEFAULT_API_BASE.into()),
            exchange_health_url: std::env::var("CLRTY_EXCHANGE_HEALTH")
                .unwrap_or_else(|_| DEFAULT_EXCHANGE_HEALTH.into()),
            chain_id: std::env::var("CLRTY_L1_CHAIN_ID")
                .unwrap_or_else(|_| CLRTY1_CHAIN_ID.into()),
            numeric_chain_id: std::env::var("CLRTY_L1_NUMERIC_CHAIN_ID")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(CLRTY1_NUMERIC_CHAIN_ID),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProbeResult {
    pub ok: bool,
    pub rpc_url: String,
    pub chain_id: String,
    pub tip_height: Option<String>,
    pub error: Option<String>,
    pub source: String,
    pub fallbacks_tried: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ConnectionReport {
    pub probe: ProbeResult,
    pub numeric_chain_id: u64,
    pub api_base: String,
    pub exchange_health_url: String,
    pub affirmed: bool,
}

fn affirms_chain(seen: &str, cfg: &Clrty1Config) -> bool {
    let raw = seen.trim();
    let lower = raw.to_lowercase();
    let expect_num = cfg.numeric_chain_id.to_string();
    let normalized = if raw.starts_with("0x") || raw.starts_with("0X") {
        u64::from_str_radix(raw.trim_start_matches("0x").trim_start_matches("0X"), 16)
            .map(|n| n.to_string())
            .unwrap_or_else(|_| raw.to_string())
    } else {
        raw.to_string()
    };
    raw == cfg.chain_id
        || lower == cfg.chain_id.to_lowercase()
        || normalized == expect_num
        || raw == expect_num
        || lower.contains("clrty-1")
}

fn text_affirms_clrty1(text: &str, cfg: &Clrty1Config) -> bool {
    let lower = text.to_lowercase();
    lower.contains("clrty-1")
        || lower.contains(&format!("\"chain\":\"{}\"", cfg.chain_id))
        || lower.contains(&format!("\"chain_id\":\"{}\"", cfg.chain_id))
        || lower.contains(&format!("\"numeric_chain_id\":{}", cfg.numeric_chain_id))
        || lower.contains(&cfg.numeric_chain_id.to_string())
}

async fn json_rpc(
    client: &reqwest::Client,
    rpc_url: &str,
    method: &str,
) -> Option<Value> {
    let body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": []
    });
    let res = client.post(rpc_url).json(&body).send().await.ok()?;
    let v: Value = res.json().await.ok()?;
    if v.get("error").is_some() {
        return None;
    }
    v.get("result").cloned()
}

async fn probe_rpc_endpoint(
    client: &reqwest::Client,
    rpc_url: &str,
    cfg: &Clrty1Config,
) -> (Option<String>, Option<String>, bool) {
    let mut seen_chain: Option<String> = None;
    for method in ["clrty_chainId", "eth_chainId", "net_version"] {
        if let Some(v) = json_rpc(client, rpc_url, method).await {
            if let Some(s) = v.as_str() {
                seen_chain = Some(s.to_string());
                break;
            }
            if let Some(n) = v.as_u64() {
                seen_chain = Some(n.to_string());
                break;
            }
        }
    }
    let mut tip: Option<String> = None;
    for method in ["clrty_blockNumber", "eth_blockNumber"] {
        if let Some(v) = json_rpc(client, rpc_url, method).await {
            if let Some(s) = v.as_str() {
                tip = Some(s.to_string());
                break;
            }
            if let Some(n) = v.as_u64() {
                tip = Some(n.to_string());
                break;
            }
        }
    }
    let ok = seen_chain
        .as_deref()
        .map(|c| affirms_chain(c, cfg))
        .unwrap_or(false);
    (seen_chain, tip, ok)
}

async fn fetch_text(client: &reqwest::Client, url: &str) -> Option<String> {
    let res = client.get(url).send().await.ok()?;
    res.text().await.ok()
}

/// Multi-endpoint probe. Fail closed only when no endpoint affirms clrty-1/1202.
pub async fn probe_clrty1(cfg: &Clrty1Config) -> Result<ProbeResult, reqwest::Error> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()?;

    let mut fallbacks_tried: Vec<String> = Vec::new();
    let api_base = cfg.api_base.trim_end_matches('/');

    // 1) Primary RPC
    fallbacks_tried.push(format!("rpc:{}", cfg.rpc_url));
    let (seen, tip, ok) = probe_rpc_endpoint(&client, &cfg.rpc_url, cfg).await;
    if ok {
        return Ok(ProbeResult {
            ok: true,
            rpc_url: cfg.rpc_url.clone(),
            chain_id: seen.unwrap_or_else(|| cfg.chain_id.clone()),
            tip_height: tip,
            error: None,
            source: "rpc".into(),
            fallbacks_tried,
        });
    }
    if let Some(ref c) = seen {
        if !affirms_chain(c, cfg) {
            return Ok(ProbeResult {
                ok: false,
                rpc_url: cfg.rpc_url.clone(),
                chain_id: c.clone(),
                tip_height: tip,
                error: Some(format!(
                    "chain_mismatch expected={}/{} got={}",
                    cfg.chain_id, cfg.numeric_chain_id, c
                )),
                source: "rpc".into(),
                fallbacks_tried,
            });
        }
    }

    // 2) RPC fallback
    if let Some(ref fb) = cfg.rpc_fallback_url {
        fallbacks_tried.push(format!("rpc_fallback:{}", fb));
        let (seen, tip, ok) = probe_rpc_endpoint(&client, fb, cfg).await;
        if ok {
            return Ok(ProbeResult {
                ok: true,
                rpc_url: fb.clone(),
                chain_id: seen.unwrap_or_else(|| cfg.chain_id.clone()),
                tip_height: tip,
                error: None,
                source: "rpc_fallback".into(),
                fallbacks_tried,
            });
        }
    }

    // 3) apiBase/rpc
    let api_rpc = format!("{}/rpc", api_base);
    fallbacks_tried.push(format!("api_rpc:{}", api_rpc));
    let (seen, tip, ok) = probe_rpc_endpoint(&client, &api_rpc, cfg).await;
    if ok {
        return Ok(ProbeResult {
            ok: true,
            rpc_url: api_rpc,
            chain_id: seen.unwrap_or_else(|| cfg.chain_id.clone()),
            tip_height: tip,
            error: None,
            source: "api_rpc".into(),
            fallbacks_tried,
        });
    }
    if let Some(text) = fetch_text(&client, &api_rpc).await {
        if text_affirms_clrty1(&text, cfg) {
            let chain = serde_json::from_str::<Value>(&text)
                .ok()
                .and_then(|v| {
                    v.get("chain")
                        .or_else(|| v.get("chain_id"))
                        .and_then(|x| x.as_str())
                        .map(|s| s.to_string())
                })
                .unwrap_or_else(|| cfg.chain_id.clone());
            return Ok(ProbeResult {
                ok: true,
                rpc_url: api_rpc,
                chain_id: chain,
                tip_height: None,
                error: None,
                source: "api_chain_affirm".into(),
                fallbacks_tried,
            });
        }
    }

    // 4) apiBase chain affirm (incl. {"error":"...","chain":"clrty-1"})
    fallbacks_tried.push(format!("api:{}", api_base));
    if let Some(text) = fetch_text(&client, api_base).await {
        if text_affirms_clrty1(&text, cfg) {
            let v: Value = serde_json::from_str(&text).unwrap_or(json!({}));
            let chain = v
                .get("chain")
                .or_else(|| v.get("chain_id"))
                .and_then(|x| x.as_str())
                .unwrap_or(&cfg.chain_id)
                .to_string();
            let tip = v
                .get("height")
                .and_then(|x| x.as_u64())
                .map(|n| n.to_string());
            return Ok(ProbeResult {
                ok: true,
                rpc_url: cfg.rpc_url.clone(),
                chain_id: chain,
                tip_height: tip,
                error: None,
                source: "api_chain_affirm".into(),
                fallbacks_tried,
            });
        }
    }

    let api_health = format!("{}/health", api_base);
    fallbacks_tried.push(format!("api_health:{}", api_health));
    if let Some(text) = fetch_text(&client, &api_health).await {
        if text_affirms_clrty1(&text, cfg) {
            let v: Value = serde_json::from_str(&text).unwrap_or(json!({}));
            let chain = v
                .get("chain")
                .or_else(|| v.get("chain_id"))
                .and_then(|x| x.as_str())
                .unwrap_or(&cfg.chain_id)
                .to_string();
            return Ok(ProbeResult {
                ok: true,
                rpc_url: cfg.rpc_url.clone(),
                chain_id: chain,
                tip_height: None,
                error: None,
                source: "api_chain_affirm".into(),
                fallbacks_tried,
            });
        }
    }

    // 5) exchange /health
    fallbacks_tried.push(format!("exchange_health:{}", cfg.exchange_health_url));
    if let Some(text) = fetch_text(&client, &cfg.exchange_health_url).await {
        if text_affirms_clrty1(&text, cfg) {
            let v: Value = serde_json::from_str(&text).unwrap_or(json!({}));
            let chain = v
                .get("chain")
                .or_else(|| v.get("chain_id"))
                .and_then(|x| x.as_str())
                .unwrap_or(&cfg.chain_id)
                .to_string();
            let tip = v
                .get("height")
                .and_then(|x| x.as_u64())
                .map(|n| n.to_string());
            return Ok(ProbeResult {
                ok: true,
                rpc_url: cfg.rpc_url.clone(),
                chain_id: chain,
                tip_height: tip,
                error: None,
                source: "exchange_health".into(),
                fallbacks_tried,
            });
        }
    }

    Ok(ProbeResult {
        ok: false,
        rpc_url: cfg.rpc_url.clone(),
        chain_id: cfg.chain_id.clone(),
        tip_height: None,
        error: Some("no_endpoint_affirmed_clrty1".into()),
        source: "none".into(),
        fallbacks_tried,
    })
}

pub async fn get_clrty1_connection_report(
    cfg: &Clrty1Config,
) -> Result<ConnectionReport, reqwest::Error> {
    let probe = probe_clrty1(cfg).await?;
    let affirmed = probe.ok;
    Ok(ConnectionReport {
        probe,
        numeric_chain_id: cfg.numeric_chain_id,
        api_base: cfg.api_base.clone(),
        exchange_health_url: cfg.exchange_health_url.clone(),
        affirmed,
    })
}

pub async fn assert_clrty1_connected(cfg: &Clrty1Config) -> Result<ProbeResult, String> {
    match probe_clrty1(cfg).await {
        Ok(p) if p.ok => Ok(p),
        Ok(p) => Err(format!(
            "CLRTY-1 not connected: {} (source={})",
            p.error.unwrap_or_else(|| "unknown".into()),
            p.source
        )),
        Err(e) => Err(e.to_string()),
    }
}

pub fn rpc_smoke_enabled() -> bool {
    std::env::var("CLRTY_RPC_SMOKE")
        .map(|v| v != "0")
        .unwrap_or(true)
}
