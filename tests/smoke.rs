use clrty_compiler_bridge::clrty1::{probe_clrty1, rpc_smoke_enabled, Clrty1Config};
use clrty_compiler_bridge::{checksum_payload, emit_stub, Ir};

#[tokio::test]
async fn emit_stub_offline() {
    std::env::set_var("CLRTY_RPC_SMOKE", "0");
    assert!(!rpc_smoke_enabled());
    let ir = Ir {
        payload: "fn main() {}".into(),
        lang: Some("rust".into()),
    };
    let out = emit_stub(&ir).await;
    assert!(out.ok);
    assert_eq!(out.checksum_sha256, checksum_payload(&ir.payload));
}

#[tokio::test]
async fn optional_rpc_smoke() {
    if std::env::var("CLRTY_RPC_SMOKE").unwrap_or_default() == "0" {
        return;
    }
    let cfg = Clrty1Config::from_env();
    let probe = probe_clrty1(&cfg).await.expect("client");
    assert!(
        probe.ok,
        "probe failed: {:?} source={} tried={:?}",
        probe.error, probe.source, probe.fallbacks_tried
    );
}
