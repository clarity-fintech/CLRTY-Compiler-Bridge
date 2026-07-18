//! Live CLRTY-1 connection smoke — prints report, exits 0/1.
use clrty_compiler_bridge::clrty1::{get_clrty1_connection_report, Clrty1Config};
use serde_json::json;

#[tokio::main]
async fn main() {
    let cfg = Clrty1Config::from_env();
    match get_clrty1_connection_report(&cfg).await {
        Ok(report) => {
            let body = json!({
                "ok": report.probe.ok,
                "rpcUrl": report.probe.rpc_url,
                "chainId": report.probe.chain_id,
                "tipHeight": report.probe.tip_height,
                "source": report.probe.source,
                "fallbacks_tried": report.probe.fallbacks_tried,
                "error": report.probe.error,
                "numericChainId": report.numeric_chain_id,
                "apiBase": report.api_base,
                "exchangeHealthUrl": report.exchange_health_url,
                "affirmed": report.affirmed,
            });
            println!("{}", serde_json::to_string_pretty(&body).unwrap());
            if report.affirmed {
                std::process::exit(0);
            }
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("{{\"ok\":false,\"error\":\"{}\"}}", e);
            std::process::exit(1);
        }
    }
}
