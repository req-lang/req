use std::{collections::HashMap, fs, io, path::PathBuf};

use clap::Parser;
use colored::Colorize;
use reqlang::{
    issue::{self, Severity, connection_errors, expr_errors, reference_errors},
    renderer::formatter::entity,
    semantics::{
        connection::{self},
        link::{self},
        typing::{self},
    },
    syntax::parser::{self},
};

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
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Command::Fmt { input, output } => {
            let source = std::fs::read_to_string(&input)?;
            let arena = parser::parse(&source)?;

            match output {
                Some(p) => entity::format(&arena, 0.into(), 0, &mut fs::File::create(p)?),
                None => entity::format(&arena, 0.into(), 0, &mut io::stdout()),
            }?;
        }
        Command::Check { input } => {
            let start = std::time::Instant::now();
            let source = fs::read_to_string(&input)?;
            let root = parser::parse(&source)?;
            let paths = link::paths(&root);
            let spans: HashMap<_, _> = paths
                .iter()
                .zip(root.iter())
                .map(|(path, node)| (path, node.payload))
                .collect();

            let resolved = link::resolve(root);
            let typed = typing::evaluate(resolved);
            let connected = connection::connect(typed);
            let elapsed = start.elapsed();

            let mut count = 0;
            for (path, node) in paths.iter().zip(connected.iter()) {
                let issues: Vec<_> = expr_errors(node)
                    .chain(connection_errors(node))
                    .chain(reference_errors(node))
                    .map(Issue::from)
                    .collect();
                count += issues.len();
                if issues.is_empty() {
                    continue;
                }

                let span = format!("{}", spans.get(&path).unwrap().start).bold();
                eprintln!("{:12} {}", span, path);

                for issue in issues {
                    eprintln!("    {}", issue.colored_message());
                }
            }

            match count {
                0 => println!("no issues ({:.2?})", elapsed),
                c => eprintln!("found {} issue(s) ({:.2?})", c, elapsed),
            }
        }
    }

    Ok(())
}

struct Issue {
    message: String,
    severity: Severity,
}

impl Issue {
    pub fn colored_message(&self) -> colored::ColoredString {
        match self.severity {
            Severity::Critical => self.message.red(),
            Severity::Severe => self.message.yellow(),
            Severity::Moderate => self.message.yellow(),
            Severity::Light => self.message.white(),
        }
    }
}

impl From<issue::Issue> for Issue {
    fn from(value: issue::Issue) -> Self {
        Self {
            message: value.to_string(),
            severity: (&value).into(),
        }
    }
}
