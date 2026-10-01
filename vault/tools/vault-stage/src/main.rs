fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match vault_stage::run(&args) {
        Ok(value) => println!("{value}"),
        Err(error) => {
            println!(
                "{}",
                serde_json::json!({"ok": false, "status": "error", "error": error})
            );
            std::process::exit(1);
        }
    }
}
