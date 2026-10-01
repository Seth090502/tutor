//! tutor-ui: minimal Rust TUI that drives `python -m tutor.cli` (no crates).
//! All teaching logic stays in Python; Rust owns the terminal loop only.

use std::env;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;

fn workspace_root() -> PathBuf {
    let mut cands: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = env::current_exe() {
        let mut p = exe.as_path();
        for _ in 0..6 {
            if let Some(par) = p.parent() {
                p = par;
                cands.push(par.to_path_buf());
            } else {
                break;
            }
        }
    }
    if let Ok(cwd) = env::current_dir() {
        let mut p = cwd.as_path();
        cands.push(cwd.clone());
        for _ in 0..4 {
            if let Some(par) = p.parent() {
                p = par;
                cands.push(par.to_path_buf());
            } else {
                break;
            }
        }
    }
    for c in cands {
        if c.join("tutor").join("cli.py").exists() {
            return c;
        }
    }
    env::current_dir().unwrap_or(PathBuf::from("."))
}

fn run_tutor(args: &[String]) -> i32 {
    let root = workspace_root();
    for py in ["python", "py"] {
        let status = Command::new(py)
            .arg("-m")
            .arg("tutor.cli")
            .args(args)
            .current_dir(&root)
            .status();
        match status {
            Ok(s) => return s.code().unwrap_or(1),
            Err(_) => continue,
        }
    }
    eprintln!("failed to launch python (tried `python` and `py`; is Python on PATH?)");
    1
}

fn read_line(prompt: &str) -> String {
    print!("{prompt}");
    let _ = io::stdout().flush();
    let mut s = String::new();
    if io::stdin().read_line(&mut s).is_err() {
        return String::new();
    }
    s.trim().to_string()
}

fn menu() -> i32 {
    loop {
        println!("\n== Tutor (Rust TUI) ==");
        println!("1 quiz  2 popquiz  3 test  4 review  5 progress  6 search  7 agent  8 eval  0 quit");
        let c = read_line("> ");
        let dom = |d: String| if d.is_empty() { "math".to_string() } else { d };
        match c.as_str() {
            "0" | "q" | "quit" => return 0,
            "1" => {
                let d = dom(read_line("domain [math]: "));
                let _ = run_tutor(&["quiz".into(), "--domain".into(), d, "--n".into(), "5".into()]);
            }
            "2" => {
                let d = dom(read_line("domain [math]: "));
                let _ = run_tutor(&["popquiz".into(), "--domain".into(), d]);
            }
            "3" => {
                let d = dom(read_line("domain [math]: "));
                let _ = run_tutor(&["test".into(), "--domain".into(), d, "--n".into(), "6".into()]);
            }
            "4" => {
                let _ = run_tutor(&["review".into(), "--n".into(), "5".into()]);
            }
            "5" => {
                let _ = run_tutor(&["progress".into()]);
            }
            "6" => {
                let q = read_line("query: ");
                if !q.is_empty() {
                    let _ = run_tutor(&["search".into(), "--q".into(), q]);
                }
            }
            "7" => {
                let d = dom(read_line("domain [math]: "));
                let _ = run_tutor(&["agent".into(), "--domain".into(), d]);
            }
            "8" => {
                let _ = run_tutor(&["eval-search".into(), "--k".into(), "3".into()]);
            }
            _ => println!("unknown choice"),
        }
    }
}

fn main() {
    let argv: Vec<String> = env::args().skip(1).collect();
    if argv.is_empty() {
        std::process::exit(menu());
    }
    std::process::exit(run_tutor(&argv));
}
