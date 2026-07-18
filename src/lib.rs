//! CLRTY Compiler Bridge — IR emit stub with CLRTY-1 probe gate.

pub mod clrty1;

use clrty1::{probe_clrty1, rpc_smoke_enabled, Clrty1Config, ProbeResult};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Intermediate representation payload for the compiler bridge.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Ir {
    /// Opaque IR text or serialized IR document.
    pub payload: String,
    /// Optional source language hint (e.g. "solidity", "move", "wat").
    #[serde(default)]
    pub lang: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmitResult {
    pub ok: bool,
    pub checksum_sha256: String,
    pub bytes: usize,
    pub probe: Option<ProbeSummary>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeSummary {
    pub ok: bool,
    pub rpc_url: String,
    pub chain_id: String,
    pub tip_height: Option<String>,
    pub error: Option<String>,
    pub source: String,
    pub fallbacks_tried: Vec<String>,
}

impl From<&ProbeResult> for ProbeSummary {
    fn from(p: &ProbeResult) -> Self {
        Self {
            ok: p.ok,
            rpc_url: p.rpc_url.clone(),
            chain_id: p.chain_id.clone(),
            tip_height: p.tip_height.clone(),
            error: p.error.clone(),
            source: p.source.clone(),
            fallbacks_tried: p.fallbacks_tried.clone(),
        }
    }
}

/// SHA-256 hex digest of IR payload bytes.
pub fn checksum_payload(payload: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(payload.as_bytes());
    hex::encode(hasher.finalize())
}

/// Emit stub: probe CLRTY-1 (unless smoke disabled), then checksum the IR payload.
///
/// When feature `llvm` is enabled, a future LLVM codegen path can replace this stub.
pub async fn emit_stub(ir: &Ir) -> EmitResult {
    #[cfg(feature = "llvm")]
    {
        // llvm feature reserved — no llvm-sys linkage in this scaffold.
        let _ = "llvm_stub";
    }

    let checksum = checksum_payload(&ir.payload);
    let bytes = ir.payload.len();

    if !rpc_smoke_enabled() {
        return EmitResult {
            ok: true,
            checksum_sha256: checksum,
            bytes,
            probe: None,
            error: None,
        };
    }

    let cfg = Clrty1Config::from_env();
    match probe_clrty1(&cfg).await {
        Ok(probe) => {
            if !probe.ok {
                return EmitResult {
                    ok: false,
                    checksum_sha256: checksum,
                    bytes,
                    probe: Some(ProbeSummary::from(&probe)),
                    error: probe
                        .error
                        .clone()
                        .or_else(|| Some("clrty1_probe_failed".into())),
                };
            }
            EmitResult {
                ok: true,
                checksum_sha256: checksum,
                bytes,
                probe: Some(ProbeSummary::from(&probe)),
                error: None,
            }
        }
        Err(e) => EmitResult {
            ok: false,
            checksum_sha256: checksum,
            bytes,
            probe: None,
            error: Some(e.to_string()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksum_is_stable() {
        let a = checksum_payload("hello");
        let b = checksum_payload("hello");
        assert_eq!(a, b);
        assert_eq!(a.len(), 64);
    }

    #[tokio::test]
    async fn emit_without_smoke() {
        std::env::set_var("CLRTY_RPC_SMOKE", "0");
        let ir = Ir {
            payload: r#"{"op":"noop"}"#.into(),
            lang: Some("json".into()),
        };
        let out = emit_stub(&ir).await;
        assert!(out.ok);
        assert!(out.probe.is_none());
        assert_eq!(out.checksum_sha256, checksum_payload(&ir.payload));
    }
}
