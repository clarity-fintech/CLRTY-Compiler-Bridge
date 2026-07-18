use clrty_compiler_bridge::{emit_stub, Ir};
use std::env;
use std::process;

#[tokio::main]
async fn main() {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() || args[0] == "--help" || args[0] == "-h" {
        eprintln!("Usage: clrty-compiler-bridge emit --ir \"<payload>\"");
        process::exit(2);
    }

    let cmd = args.remove(0);
    if cmd != "emit" {
        eprintln!("Unknown command: {cmd}");
        eprintln!("Usage: clrty-compiler-bridge emit --ir \"<payload>\"");
        process::exit(2);
    }

    let mut ir_payload: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--ir" => {
                if i + 1 >= args.len() {
                    eprintln!("--ir requires a value");
                    process::exit(2);
                }
                ir_payload = Some(args[i + 1].clone());
                i += 2;
            }
            other => {
                eprintln!("Unknown flag: {other}");
                process::exit(2);
            }
        }
    }

    let Some(payload) = ir_payload else {
        eprintln!("Missing --ir \"...\"");
        process::exit(2);
    };

    let ir = Ir {
        payload,
        lang: None,
    };

    let result = emit_stub(&ir).await;
    println!("{}", serde_json::to_string_pretty(&result).unwrap());
    if !result.ok {
        process::exit(1);
    }
}
