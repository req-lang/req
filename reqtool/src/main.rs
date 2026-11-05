use std::{fs, io, path::PathBuf};

use clap::Parser;
use colored::Colorize;
use reqlang::{
    Specification,
    issue::{self, Severity},
    renderer::formatter,
    syntax::{location::Span, parser},
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
            let (tree, _, errors) = parser::parse(&source);

            // The tree is incomplete when there are syntax errors, formatting it would lose code
            if !errors.is_empty() {
                report_syntax_errors(&source, &errors);
                std::process::exit(1);
            }

            match output {
                Some(p) => formatter::format(&tree, &mut fs::File::create(p)?),
                None => formatter::format(&tree, &mut io::stdout()),
            }?;
        }
        Command::Check { input } => {
            let start = std::time::Instant::now();
            let source = std::fs::read_to_string(&input)?;
            let (tree, spans, errors) = parser::parse(&source);
            report_syntax_errors(&source, &errors);
            let mut spec = Specification::from(tree);
            spec.diagnose();
            let issues = issue::issues(&spec);

            let elapsed = start.elapsed();
            let mut count = errors.len();

            for ((issues, path), span) in issues
                .into_iter()
                .zip(spec.analysis.paths)
                .zip(spans.entities)
            {
                count += issues.len();
                if issues.is_empty() {
                    continue;
                }

                let sp = format!("{}", span.start).bold();
                let code = format!("{}", &source[span.start.index..span.end.index]).bold();
                eprintln!("{:12} {} ---> {}", sp, path.path, code);

                for issue in issues.into_iter().map(Issue::from) {
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

fn report_syntax_errors(source: &str, errors: &[Span]) {
    for span in errors {
        let end = span.end.index.min(source.len());
        let start = span.start.index.min(end);
        let code = source[start..end].bold();
        eprintln!("{:12} {}", format!("{}", span.start).bold(), "syntax error".red());
        if !code.is_empty() {
            eprintln!("    {}", code);
        }
    }
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
