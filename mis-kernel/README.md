# CLRTY-MIS-Kernel (`misc`)

Sole active **MIS** compiler kernel for CLRTY-1 / chain **1202**.

Foreign kernels (`python3 … clrtyc.py`, `solc`, `forge`, `hardhat`, …) are **hard-refused (exit 3)**.

## Kernel download / install

```bash
git clone https://github.com/clarity-fintech/CLRTY-MIS-Kernel.git
cd CLRTY-MIS-Kernel
bash scripts/download_misc_kernel.sh
./bin/misc path.mis --check --compact-letters
```

Prebuilt (this package): `bin/misc-darwin-arm64` — checksums in `bin/SHA256SUMS.txt`.

From source:

```bash
cd src/misc && cargo build --release
cp target/release/misc ../../bin/misc
```

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
