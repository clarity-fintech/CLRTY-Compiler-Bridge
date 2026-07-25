# CLRTY-MIS-Kernel (`misc`)

Sole active **MIS** compiler kernel for CLRTY-1 / chain **1202**.

Foreign kernels (`python3 … clrtyc.py`, `solc`, `forge`, `hardhat`, …) are **hard-refused (exit 3)**.

## Install + smoke test

Clone into any empty directory (not inside an existing `CLRTY-MIS-Kernel` tree):

```bash
git clone https://github.com/clarity-fintech/CLRTY-MIS-Kernel.git
cd CLRTY-MIS-Kernel
bash scripts/download_misc_kernel.sh
./bin/misc src/misc.mis --check --compact-letters
```

`path.mis` in older docs was a placeholder — use `src/misc.mis` (shipped) or your own `.mis` file.

Prebuilt (this package): `bin/misc-darwin-arm64` — checksums in `bin/SHA256SUMS.txt`.

From source:

```bash
cd src/misc && cargo build --release
cp target/release/misc ../../bin/misc
```

## Monorepo maintainers (not this clone)

`make sync-mis-kernel` / `make push-mis-kernel` live in the **CLRTY project root**, not inside this kernel repo.

## Modules

| File | Role |
|------|------|
| `src/misc.mis` | Kernel surface |
| `src/mis_kernel.mis` | Native packs 776..=875 |
| `manifests/mis_kernel_active_only.json` | Active-only policy |

## Policy

- Active kernel: **`misc` only**
- Extension: **`.mis`** (`.clrty` legacy)
- Settlement: **clrty-1 / 1202**
