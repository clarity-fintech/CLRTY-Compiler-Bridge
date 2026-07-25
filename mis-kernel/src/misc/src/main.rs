//! misc — MIS kernel compiler (canonical). Not Python.
//! Compiles `.mis` (`.clrty` / `.crty` legacy) → execution graph + letter-hash + EMBED 3..=6.
//! Settlement: clrty-1 / chain 1202. Kernel surface: MisNativeKernels / mis_kernel.

use clap::Parser;
use regex::Regex;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

const SETTLEMENT_CHAIN_ID: u64 = 1202;
const SETTLEMENT_NETWORK: &str = "clrty-1";
const LETTER_ALGO: &str = "sha256-rolling-typed-letter/v1";
/// Sole active MIS compiler kernel. Any other kernel is a hard error.
const ACTIVE_KERNEL: &str = "misc";

#[derive(Parser, Debug)]
#[command(
    name = "misc",
    about = "MIS kernel ONLY — compile/check .mis. Foreign kernels rejected. Alias: clrtyc→misc"
)]
struct Args {
    input: PathBuf,
    #[arg(short, long)]
    output: Option<PathBuf>,
    #[arg(long)]
    check: bool,
    #[arg(long)]
    compact_letters: bool,
    #[arg(long, default_value_t = true)]
    require_embed: bool,
    #[arg(long)]
    allow_missing_embed: bool,
}

#[derive(Debug)]
struct ForeignKernelHit {
    kernel: &'static str,
    evidence: String,
}

/// Reject env / argv that try to activate a non-misc kernel.
fn enforce_active_kernel_only() -> Result<(), String> {
    for key in [
        "MIS_KERNEL",
        "CLRTY_MIS_KERNEL",
        "CLRTY_COMPILER_KERNEL",
        "CLRTY_ACTIVE_KERNEL",
    ] {
        if let Ok(v) = std::env::var(key) {
            let t = v.trim();
            if !t.is_empty() && !t.eq_ignore_ascii_case(ACTIVE_KERNEL) {
                return Err(format!(
                    "foreign kernel refused: env {key}={t:?} — only active kernel is `{ACTIVE_KERNEL}`"
                ));
            }
        }
    }
    if let Ok(v) = std::env::var("CLRTY_COMPILER") {
        let t = v.trim().to_ascii_lowercase();
        if !t.is_empty()
            && !t.contains("misc")
            && (t.contains("python")
                || t.contains("clrtyc.py")
                || t.contains("solc")
                || t.contains("forge")
                || t.contains("hardhat")
                || t.contains("llvm"))
        {
            return Err(format!(
                "foreign kernel refused: env CLRTY_COMPILER={v:?} — use bin/misc only"
            ));
        }
    }
    Ok(())
}

/// Scan source for directives / invocations of non-misc kernels.
fn detect_foreign_kernels(text: &str) -> Vec<ForeignKernelHit> {
    let mut hits = Vec::new();

    // Plain patterns (regex crate — no lookaround).
    let plain: &[(&str, &str)] = &[
        (r"(?i)python3\s+\S*clrtyc\.py", "python3-clrtyc"),
        (r"(?i)\bpragma\s+solidity\b", "solidity"),
        (r"(?i)\bforge\s+create\b", "foundry"),
        (r"(?i)\bsolc\s+", "solc"),
        (r"(?i)\bhardhat\s+(compile|run)\b", "hardhat"),
    ];
    for (pat, kernel) in plain {
        let re = Regex::new(pat).expect("foreign-kernel regex");
        if let Some(m) = re.find(text) {
            let snippet: String = m.as_str().chars().take(80).collect();
            hits.push(ForeignKernelHit {
                kernel,
                evidence: snippet,
            });
        }
    }

    // Declared kernel fields — capture value, reject if not misc.
    let declared: &[(&str, &str)] = &[
        (
            r#"(?i)compiler_kernel\s*[=:]\s*["']?([a-z0-9_.-]+)"#,
            "declared-foreign-kernel",
        ),
        (
            r#"(?i)active_kernel\s*[=:]\s*["']?([a-z0-9_.-]+)"#,
            "declared-foreign-kernel",
        ),
        (
            r#"(?i)\bkernel\s*[:=]\s*["']([a-z0-9_.-]+)["']"#,
            "declared-foreign-kernel",
        ),
    ];
    for (pat, kernel) in declared {
        let re = Regex::new(pat).expect("declared-kernel regex");
        for caps in re.captures_iter(text) {
            let val = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            if !val.eq_ignore_ascii_case(ACTIVE_KERNEL) {
                hits.push(ForeignKernelHit {
                    kernel,
                    evidence: format!("{val}"),
                });
            }
        }
    }
    hits
}

fn reject_foreign(hits: &[ForeignKernelHit]) -> ExitCode {
    eprintln!(
        "error: foreign kernel refused — only active MIS kernel is `{ACTIVE_KERNEL}`"
    );
    for h in hits {
        eprintln!("  - kernel={} evidence={:?}", h.kernel, h.evidence);
    }
    eprintln!("  fix: use `bin/misc path.mis --check --compact-letters`");
    ExitCode::from(3)
}

fn is_typed_letter(ch: char) -> bool {
    if ch == '\r' || ch == '\n' || ch == '\t' {
        return false;
    }
    let o = ch as u32;
    o >= 32 && o != 127
}

fn hash_typed_letters(text: &str, include_detail: bool) -> Value {
    let mut rolling = [0u8; 32];
    let mut typed_i: u64 = 0;
    let mut letters = Vec::new();
    for (abs_i, ch) in text.char_indices() {
        if !is_typed_letter(ch) {
            continue;
        }
        let mut hasher = Sha256::new();
        hasher.update(rolling);
        hasher.update((abs_i as u64).to_le_bytes());
        hasher.update(typed_i.to_le_bytes());
        let mut buf = [0u8; 4];
        let encoded = ch.encode_utf8(&mut buf);
        hasher.update(encoded.as_bytes());
        let digest = hasher.finalize();
        rolling.copy_from_slice(&digest);
        if include_detail {
            letters.push(json!({
                "i": typed_i,
                "abs": abs_i,
                "char": ch.to_string(),
                "cp": ch as u32,
                "h": hex::encode(digest),
            }));
        }
        typed_i += 1;
    }
    let mut out = json!({
        "algorithm": LETTER_ALGO,
        "typed_letter_count": typed_i,
        "root": hex::encode(rolling),
    });
    if include_detail {
        out["letters"] = Value::Array(letters);
    }
    out
}

fn parse_embed(text: &str) -> Value {
    let embed_re = Regex::new(r"(?is)embed\s+(?:gates?\s+)?3\s*\.\.=\s*6\s*\{([^}]*)\}").unwrap();
    let field_re = Regex::new(r#"(?i)(context|ai|execution|capital)\s*:\s*"([^"]*)"\s*;?"#).unwrap();
    let mut fields = std::collections::HashMap::from([
        ("context".to_string(), String::new()),
        ("ai".to_string(), String::new()),
        ("execution".to_string(), String::new()),
        ("capital".to_string(), String::new()),
    ]);
    let present = if let Some(m) = embed_re.captures(text) {
        for fm in field_re.captures_iter(&m[1]) {
            fields.insert(fm[1].to_lowercase(), fm[2].to_string());
        }
        true
    } else {
        false
    };

    let gate_names = [(3, "context"), (4, "ai"), (5, "execution"), (6, "capital")];
    let mut layers = Vec::new();
    let mut concat = String::new();
    for (g, name) in gate_names {
        let intent = fields.get(name).cloned().unwrap_or_default();
        let mut h = Sha256::new();
        h.update(format!("gate{g}:{name}:{intent}").as_bytes());
        let layer_hash = hex::encode(h.finalize());
        concat.push_str(&layer_hash);
        layers.push(json!({
            "gate": g,
            "name": name,
            "intent": intent,
            "layer_hash": layer_hash,
        }));
    }
    let mut er = Sha256::new();
    er.update(concat.as_bytes());
    json!({
        "schema": "clrty.mis_ml.embed_gates_3_6/v1",
        "required": true,
        "gates": "3..=6",
        "layers": layers,
        "embed_root": hex::encode(er.finalize()),
        "present_in_source": present,
    })
}

fn slice_brace_body(text: &str, start: usize) -> String {
    let bytes = text.as_bytes();
    let mut depth = 1i32;
    let mut i = start;
    while i < bytes.len() && depth > 0 {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => depth -= 1,
            _ => {}
        }
        i += 1;
    }
    text[start..i.saturating_sub(1)].to_string()
}

fn compile_source(text: &str, path: &str, include_letter_detail: bool) -> Value {
    let module_re = Regex::new(r"(?m)^\s*module\s+(\w+)\s*\{").unwrap();
    let invariant_re = Regex::new(r"(?m)^\s*invariant\s+(\w+)\s*:\s*(.+?)\s*;\s*$").unwrap();
    let outcome_re = Regex::new(r"(?m)^\s*outcome\s+(\w+)\s*\(([^)]*)\)\s*\{").unwrap();
    let require_re = Regex::new(r"(?m)^\s*require\s+(.+?)\s*;\s*$").unwrap();
    let constraint_re = Regex::new(r"(?m)^\s*constraint\s+(.+?)\s*;\s*$").unwrap();

    let modules: Vec<String> = module_re
        .captures_iter(text)
        .map(|c| c[1].to_string())
        .collect();
    let invariants: Vec<Value> = invariant_re
        .captures_iter(text)
        .map(|c| json!({"id": &c[1], "expr": c[2].trim()}))
        .collect();

    let mut outcomes = Vec::new();
    for m in outcome_re.captures_iter(text) {
        let name = m[1].to_string();
        let params: Vec<String> = m[2]
            .split(',')
            .map(|p| p.trim().to_string())
            .filter(|p| !p.is_empty())
            .collect();
        let body = slice_brace_body(text, m.get(0).unwrap().end());
        let requires: Vec<String> = require_re
            .captures_iter(&body)
            .map(|c| c[1].trim().to_string())
            .collect();
        let constraints: Vec<String> = constraint_re
            .captures_iter(&body)
            .map(|c| c[1].trim().to_string())
            .collect();
        let mut bh = Sha256::new();
        bh.update(body.as_bytes());
        let body_hash = hex::encode(bh.finalize())[..16].to_string();
        outcomes.push(json!({
            "name": name,
            "params": params,
            "requires": requires,
            "constraints": constraints,
            "body_hash": body_hash,
        }));
    }

    let letter = hash_typed_letters(text, include_letter_detail);
    let embed = parse_embed(text);
    let mut dh = Sha256::new();
    dh.update(text.as_bytes());
    let digest = hex::encode(dh.finalize());
    let letter_root = letter["root"].as_str().unwrap_or("");
    let embed_root = embed["embed_root"].as_str().unwrap_or("");
    let mut deep = Sha256::new();
    deep.update(format!("{letter_root}{embed_root}{digest}").as_bytes());
    let deep_root_digest = hex::encode(deep.finalize());

    let module = modules
        .first()
        .cloned()
        .unwrap_or_else(|| {
            PathBuf::from(path)
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "unknown".into())
        });

    json!({
        "schema": "clrty.mis_ml.execution_graph/v2",
        "source": path,
        "module": module,
        "language": "mis-ml",
        "kernel": ACTIVE_KERNEL,
        "active_kernel_only": true,
        "foreign_kernels_rejected": true,
        "deep_root": "moniversive",
        "extension": ".mis",
        "legacy_extensions": [".clrty", ".crty"],
        "settlement_chain_id": SETTLEMENT_CHAIN_ID,
        "settlement_network": SETTLEMENT_NETWORK,
        "invariants": invariants,
        "outcomes": outcomes,
        "letter_hash": letter,
        "embed": embed,
        "deep_root_digest": deep_root_digest,
        "source_sha256": digest,
        "invariant_count": invariants.len(),
        "outcome_count": outcomes.len(),
        "principles": {
            "code": "intent",
            "execution": "optimized_outcome",
            "state": "continuously_verified_invariants",
            "typed_letters": "individually_hashed",
            "embed_gates": "3..=6",
            "compiler": ACTIVE_KERNEL,
            "active_kernel_only": true,
        },
    })
}

fn main() -> ExitCode {
    if let Err(e) = enforce_active_kernel_only() {
        eprintln!("error: {e}");
        return ExitCode::from(3);
    }

    let args = Args::parse();
    let suffix = args
        .input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    if !matches!(suffix, "mis" | "clrty" | "crty") {
        eprintln!(
            "error: expected .mis (canonical) or .clrty legacy alias (got .{suffix})"
        );
        eprintln!("  foreign formats are not an active MIS kernel — use .mis with bin/misc");
        return ExitCode::from(2);
    }

    let text = match fs::read_to_string(&args.input) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: read {}: {e}", args.input.display());
            return ExitCode::from(1);
        }
    };

    let foreign = detect_foreign_kernels(&text);
    if !foreign.is_empty() {
        return reject_foreign(&foreign);
    }

    let path = args.input.display().to_string();
    let graph = compile_source(&text, &path, !args.compact_letters);

    let module = graph["module"].as_str().unwrap_or("");
    if module.is_empty() {
        eprintln!("error: no module declaration");
        return ExitCode::from(1);
    }
    let inv = graph["invariant_count"].as_u64().unwrap_or(0);
    let outc = graph["outcome_count"].as_u64().unwrap_or(0);
    if inv == 0 && outc == 0 {
        eprintln!("error: module has no invariants or outcomes");
        return ExitCode::from(1);
    }
    let present = graph["embed"]["present_in_source"].as_bool().unwrap_or(false);
    if !args.allow_missing_embed && args.require_embed && !present {
        eprintln!(
            "error: missing `embed gates 3..=6 {{ ... }}` — Moniversive deep-root requires EMBED"
        );
        return ExitCode::from(1);
    }
    let typed = graph["letter_hash"]["typed_letter_count"]
        .as_u64()
        .unwrap_or(0);
    if typed == 0 {
        eprintln!("error: no typed letters to hash");
        return ExitCode::from(1);
    }

    if args.check {
        let lr = graph["letter_hash"]["root"].as_str().unwrap_or("");
        let er = graph["embed"]["embed_root"].as_str().unwrap_or("");
        let dr = graph["deep_root_digest"].as_str().unwrap_or("");
        let trunc = |s: &str| {
            if s.len() >= 16 {
                format!("{}…", &s[..16])
            } else {
                format!("{s}…")
            }
        };
        println!(
            "{}",
            json!({
                "ok": true,
                "kernel": ACTIVE_KERNEL,
                "active_kernel_only": true,
                "module": module,
                "invariant_count": inv,
                "outcome_count": outc,
                "typed_letters": typed,
                "letter_root": trunc(lr),
                "embed_root": trunc(er),
                "deep_root_digest": trunc(dr),
                "embed_present": present,
            })
        );
        return ExitCode::SUCCESS;
    }

    let out = format!("{}\n", serde_json::to_string_pretty(&graph).unwrap());
    if let Some(path) = args.output {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Err(e) = fs::write(&path, &out) {
            eprintln!("error: write {}: {e}", path.display());
            return ExitCode::from(1);
        }
        println!("wrote {}", path.display());
    } else {
        print!("{out}");
    }
    ExitCode::SUCCESS
}
