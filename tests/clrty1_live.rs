use clrty_compiler_bridge::clrty1::{
    get_clrty1_connection_report, probe_clrty1, Clrty1Config, CLRTY1_CHAIN_ID,
    CLRTY1_NUMERIC_CHAIN_ID,
};

fn live_enabled() -> bool {
    std::env::var("CLRTY_LIVE").unwrap_or_else(|_| "1".into()) != "0"
}

#[tokio::test]
async fn live_probe_affirms_clrty1_via_failover() {
    if !live_enabled() {
        return;
    }

    let cfg = Clrty1Config::from_env();
    let probe = probe_clrty1(&cfg).await.expect("http client");
    assert!(
        probe.ok,
        "probe failed: {:?} source={} tried={:?}",
        probe.error, probe.source, probe.fallbacks_tried
    );
    assert!(
        probe.chain_id == CLRTY1_CHAIN_ID
            || probe.chain_id == CLRTY1_NUMERIC_CHAIN_ID.to_string()
            || probe.chain_id.to_lowercase().contains("clrty-1"),
        "unexpected chain_id {}",
        probe.chain_id
    );
    assert!(!probe.fallbacks_tried.is_empty());

    let report = get_clrty1_connection_report(&cfg).await.expect("report");
    assert!(report.affirmed);
    assert_eq!(report.numeric_chain_id, CLRTY1_NUMERIC_CHAIN_ID);
}
