use std::{fs, io, path::PathBuf};

use clap::Parser;
use colored::Colorize;
use itertools::Itertools;
use reqdiag::Diagnostic;
use reqlang::{
    Specification, exporter,
    renderer::formatter,
    syntax::{location::Span, parser},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    match Cli::parse().command {
        Command::Fmt { input, output } => {
            let source = std::fs::read_to_string(&input)?;
            let (tree, info) = parser::parse(&source);
            let errors = info.errors;

            if !errors.is_empty() {
                return Err(syntax_error_message(&errors).into());
            }

            match output {
                Some(p) => formatter::format(&tree, &mut fs::File::create(p)?),
                None => formatter::format(&tree, &mut io::stdout()),
            }?;
        }
        Command::Export { input, output } => {
            let source = std::fs::read_to_string(&input)?;
            let (tree, info) = parser::parse(&source);
            let errors = info.errors;

            if !errors.is_empty() {
                return Err(syntax_error_message(&errors).into());
            }

            let spec = Specification::from(tree);
            match output {
                Some(p) => exporter::html::render(&spec, &mut fs::File::create(p)?),
                None => exporter::html::render(&spec, &mut io::stdout()),
            }?;
        }
        Command::Check { input, json } => {
            let source = std::fs::read_to_string(&input)?;
            let (tree, info) = parser::parse(&source);
            let mut spec = Specification::from(tree);
            spec.diagnose();
            let diagnostics = Diagnostic::collect(&spec, &info, &source);

            let out = match json {
                true => serde_json::to_string_pretty(&diagnostics)?,
                false => pretty_diagnostics(diagnostics),
            };
            println!("{}", out);
        }
    }

    Ok(())
}

fn syntax_error_message(errors: &[Span]) -> String {
    format!(
        "failed, {} syntax error(s), run `reqtool check` to see them",
        errors.len()
    )
}

fn pretty_diagnostics(diagnostics: Vec<Diagnostic>) -> String {
    diagnostics.into_iter().map(pretty_diagnostic).join("\n")
}

fn pretty_diagnostic(diagnostic: Diagnostic) -> String {
    let sp = diagnostic.span.start.to_string().on_black();
    let code = diagnostic.code.dimmed();
    let msg = match diagnostic.severity {
        "critical" => diagnostic.message.on_red(),
        "severe" => diagnostic.message.red(),
        "moderate" => diagnostic.message.yellow(),
        "light" => diagnostic.message.white(),
        _ => unreachable!(),
    };
    let snp = diagnostic.snippet;

    format!("{:12} {} {}\n{}", sp, code, msg, snp)
}

#[derive(clap::Parser)]
#[command(name = "reqtool", about = "Tooling for the req language")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Format a req source file
    Fmt {
        /// Input file
        input: PathBuf,
        /// Output file (stdout by default)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Check a req source file for errors
    Check {
        /// Input file
        input: PathBuf,
        /// Output the report as JSON on stdout
        #[arg(long)]
        json: bool,
    },
    /// Export a req source file as an HTML fragment, to put in a page
    Export {
        /// Input file
        input: PathBuf,
        /// Output file (stdout by default)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}
