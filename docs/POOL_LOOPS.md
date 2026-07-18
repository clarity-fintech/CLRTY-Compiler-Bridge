# CLRTY-1 pool loops (Compiler-Bridge)

This Rust crate does not embed the TypeScript `pool_loops` module. Liquidity
add/remove/rebalance/quote loops live in the TS service repos under
`src/liquidity/pool_loops.ts`.

Compiler-Bridge responsibilities related to pools:

1. **Probe before emit** — `emit_stub` calls `probe_clrty1` when `CLRTY_RPC_SMOKE`
   is enabled so IR artifacts are only produced against a reachable CLRTY-1 tip.
2. **eBPF settlement path** — see `security/ebpf/filters.yaml` (`deny_by_default`)
   and `security/CHECKLIST.md` for the outbound allowlist that protects RPC/API
   egress used by pool finalize hooks elsewhere in the stack.
3. **Chain pin** — `clrty-1` / numeric `1202` / denom `uclrty`.

Pool finalize HTTP hooks target `CLRTY_API_BASE/v1/pools/finalize` from TS services;
this bridge remains the IR emit gate only.
