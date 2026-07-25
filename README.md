# CLRTY-Compiler-Bridge


## MIS kernel (`misc`) — required

Sole active CLRTY-1 / Moniversive compiler kernel. **Not Python.**

```bash
# Download from clarity-fintech/CLRTY-MIS-Kernel
git clone https://github.com/clarity-fintech/CLRTY-MIS-Kernel.git
cd CLRTY-MIS-Kernel && bash scripts/download_misc_kernel.sh
./bin/misc path.mis --check --compact-letters
```

Or from the Developer Kit: [`dist/mis-kernel-misc.zip`](https://github.com/clarity-fintech/developer_kit/raw/main/dist/mis-kernel-misc.zip)

Policy: foreign kernels (`python3 clrtyc`, `solc`, `forge`, `hardhat`) → **hard error**. Settlement **clrty-1 / 1202**.

Rust compiler bridge for **CLRTY-1**: accepts IR payloads, probes the L1 RPC (optional), and emits a SHA-256 checksum stub artifact.

## Features

- Default build: Tokio + reqwest (rustls) + sha2 — no LLVM required
- Optional Cargo feature `llvm` — reserved stub (no `llvm-sys` linkage yet)
- Embedded CLRTY-1 client (`src/clrty1.rs`)
- **Probe before emit** — `emit_stub` fails closed when `CLRTY_RPC_SMOKE=1` and CLRTY-1 is unreachable
- **eBPF settlement-path stubs** — `security/ebpf/filters.yaml` (`deny_by_default`) plus `settlement_path.bpf.c` (CI validates YAML only; no kernel load)

## Security

- Ops checklist: [`security/CHECKLIST.md`](./security/CHECKLIST.md)
- eBPF allowlist: [`security/ebpf/`](./security/ebpf/)
- Pool-loop notes (TS services own the loops): [`docs/POOL_LOOPS.md`](./docs/POOL_LOOPS.md)
- Skill manifest: [`manifests/skill.json`](./manifests/skill.json) (`CLRTY-CB-001`, substrate `CLRTY-1`)

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
