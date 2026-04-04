mod args;
mod commands;
mod util;

use args::{Cli, Commands};
use clap::Parser;

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Compile {
            input,
            format,
            output,
            input_format,
            view,
        } => {
            commands::compile::handle_compile(input, format, output, input_format, view);
        }
        Commands::Import {
            input,
            output,
            strip_chrome,
            infer_semantics,
            infer_llm,
        } => {
            commands::compile::handle_import(input, output, strip_chrome, infer_semantics, infer_llm);
        }
        Commands::DumpIr { input } => {
            commands::compile::handle_dump_ir(input);
        }
        Commands::Schema {} => {
            commands::compile::handle_schema();
        }
        Commands::Skill { action } => {
            commands::skill::handle_skill(action);
        }
        Commands::Chunk { action } => {
            commands::chunk::handle_chunk(action);
        }
        Commands::Config { action } => {
            commands::config::handle_config(action);
        }
        Commands::Migrate { action } => {
            commands::migrate::handle_migrate(action);
        }
        Commands::Lint { input, format } => {
            commands::lint::handle_lint(input, format);
        }
        Commands::Check { input, format } => {
            commands::lint::handle_check(input, format);
        }
        Commands::Observe {
            skill,
            output,
            format,
        } => {
            commands::observe::handle_observe(skill, output, format);
        }
        Commands::Conflict { files, format } => {
            commands::conflict::handle_conflict(files, format);
        }
        Commands::Scan { input, format } => {
            commands::scan::handle_scan(input, format);
        }
        Commands::MigrateSyntax {
            path,
            in_place,
            dry_run,
        } => {
            commands::migrate_syntax::handle_migrate_syntax(path, in_place, dry_run);
        }
    }
}
