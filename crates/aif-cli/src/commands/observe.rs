use std::path::PathBuf;

use crate::util::{parse_aif, read_source};

pub fn handle_observe(skill: PathBuf, output: PathBuf, format: String) {
    let source = read_source(&skill);
    let doc = parse_aif(&source);

    let llm_output = if output.to_str() == Some("-") {
        use std::io::Read;
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .unwrap_or_else(|e| {
                eprintln!("Error reading stdin: {}", e);
                std::process::exit(1);
            });
        buf
    } else {
        read_source(&output)
    };

    match aif_observe::report::observe(&doc, &llm_output) {
        Ok(report) => {
            let result = match format.as_str() {
                "json" => serde_json::to_string_pretty(&report).unwrap(),
                _ => aif_observe::report::format_text(&report),
            };
            print!("{}", result);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
