use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "aif")]
#[command(about = "AIF: AI-native Interchange Format compiler")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile an AIF document to an output format
    Compile {
        /// Input .aif file
        input: PathBuf,
        /// Output format: html, markdown, lml, json
        #[arg(short, long, default_value = "html")]
        format: String,
        /// Output file (defaults to stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Import a Markdown file to AIF IR (JSON)
    Import {
        /// Input Markdown file
        input: PathBuf,
        /// Output file (defaults to stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Dump the parsed IR as JSON
    DumpIr {
        /// Input .aif file
        input: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Compile { input, format, output } => {
            let source = fs::read_to_string(&input).unwrap_or_else(|e| {
                eprintln!("Error reading {}: {}", input.display(), e);
                std::process::exit(1);
            });
            let doc = aif_parser::parse(&source).unwrap_or_else(|errors| {
                for e in &errors {
                    eprintln!("{}", e);
                }
                std::process::exit(1);
            });

            let result = match format.as_str() {
                "html" => aif_html::render_html(&doc),
                "markdown" | "md" => aif_markdown::render_markdown(&doc),
                "lml" => aif_lml::render_lml(&doc),
                "json" => serde_json::to_string_pretty(&doc).unwrap(),
                _ => {
                    eprintln!("Unknown format: {}. Supported: html, markdown, lml, json", format);
                    std::process::exit(1);
                }
            };

            if let Some(output_path) = output {
                fs::write(&output_path, &result).unwrap_or_else(|e| {
                    eprintln!("Error writing {}: {}", output_path.display(), e);
                    std::process::exit(1);
                });
                eprintln!("Wrote {}", output_path.display());
            } else {
                print!("{}", result);
            }
        }
        Commands::Import { input, output } => {
            let source = fs::read_to_string(&input).unwrap_or_else(|e| {
                eprintln!("Error reading {}: {}", input.display(), e);
                std::process::exit(1);
            });
            let doc = aif_markdown::import_markdown(&source);
            let json = serde_json::to_string_pretty(&doc).unwrap();

            if let Some(output_path) = output {
                fs::write(&output_path, &json).unwrap_or_else(|e| {
                    eprintln!("Error writing {}: {}", output_path.display(), e);
                    std::process::exit(1);
                });
                eprintln!("Wrote {}", output_path.display());
            } else {
                print!("{}", json);
            }
        }
        Commands::DumpIr { input } => {
            let source = fs::read_to_string(&input).unwrap_or_else(|e| {
                eprintln!("Error reading {}: {}", input.display(), e);
                std::process::exit(1);
            });
            let doc = aif_parser::parse(&source).unwrap_or_else(|errors| {
                for e in &errors {
                    eprintln!("{}", e);
                }
                std::process::exit(1);
            });
            let json = serde_json::to_string_pretty(&doc).unwrap();
            println!("{}", json);
        }
    }
}
