//! CLRTY-1 L1 RPC client (duplicated into Compiler-Bridge).

use serde_json::{json, Value};

pub const CLRTY1_CHAIN_ID: &str = "clrty-1";
pub const CLRTY1_NUMERIC_CHAIN_ID: u64 = 1202;
pub const CLRTY1_DENOM: &str = "uclrty";

pub struct Clrty1Config {
    pub rpc_url: String,
    pub api_base: String,
    pub chain_id: String,
    pub numeric_chain_id: u64,
}

impl Clrty1Config {
    pub fn from_env() -> Self {
        Self {
            rpc_url: std::env::var("CLRTY_L1_RPC")
                .or_else(|_| std::env::var("CLRTY_L1_RPC_URL"))
                .unwrap_or_else(|_| "https://rpc.clarity-fintech.com".into()),
            api_base: std::env::var("CLRTY_API_BASE")
                .unwrap_or_else(|_| "https://api.clarity-fintech.com".into()),
            chain_id: std::env::var("CLRTY_L1_CHAIN_ID")
                .unwrap_or_else(|_| CLRTY1_CHAIN_ID.into()),
            numeric_chain_id: std::env::var("CLRTY_L1_NUMERIC_CHAIN_ID")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(CLRTY1_NUMERIC_CHAIN_ID),
        }
    }
}

pub struct ProbeResult {
    pub ok: bool,
    pub rpc_url: String,
    pub chain_id: String,
    pub tip_height: Option<String>,
    pub error: Option<String>,
}

pub async fn probe_clrty1(cfg: &Clrty1Config) -> Result<ProbeResult, reqwest::Error> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "eth_blockNumber",
        "params": []
    });

    let res = client.post(&cfg.rpc_url).json(&body).send().await;
    match res {
        Ok(r) => {
            let status = r.status();
            let v: Value = r.json().await.unwrap_or(json!({}));
            let tip = v
                .get("result")
                .and_then(|x| x.as_str())
                .map(|s| s.to_string());
            if status.is_success() || tip.is_some() {
                Ok(ProbeResult {
                    ok: true,
                    rpc_url: cfg.rpc_url.clone(),
                    chain_id: cfg.chain_id.clone(),
                    tip_height: tip,
                    error: None,
                })
            } else {
                Ok(ProbeResult {
                    ok: false,
                    rpc_url: cfg.rpc_url.clone(),
                    chain_id: cfg.chain_id.clone(),
                    tip_height: None,
                    error: Some(format!("http_{}", status.as_u16())),
                })
            }
        }
        Err(e) => Ok(ProbeResult {
            ok: false,
            rpc_url: cfg.rpc_url.clone(),
            chain_id: cfg.chain_id.clone(),
            tip_height: None,
            error: Some(e.to_string()),
        }),
    }
}

pub fn rpc_smoke_enabled() -> bool {
    std::env::var("CLRTY_RPC_SMOKE").map(|v| v != "0").unwrap_or(true)
}
