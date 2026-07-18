# CLRTY-Compiler-Bridge

Rust compiler bridge for **CLRTY-1**: accepts IR payloads, probes the L1 RPC (optional), and emits a SHA-256 checksum stub artifact.

## Features

- Default build: Tokio + reqwest (rustls) + sha2 — no LLVM required
- Optional Cargo feature `llvm` — reserved stub (no `llvm-sys` linkage yet)
- Embedded CLRTY-1 client (`src/clrty1.rs`)

## Usage

```bash
export CLRTY_RPC_SMOKE=0
cargo test
cargo run -- emit --ir '{"op":"noop"}'
```

With live probe:

```bash
export CLRTY_RPC_SMOKE=1
export CLRTY_L1_RPC=https://rpc.clarity-fintech.com
cargo run -- emit --ir '{"op":"noop"}'
```

## Environment

See `.env.example`.

## License

Apache-2.0 — see [LICENSE](./LICENSE).
