use std::{fs, io, path::PathBuf};

use clap::Parser;
use colored::Colorize;
use reqlang::{
    Specification,
    issue::{self, Severity},
    renderer::formatter,
    syntax::parser,
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
            let (tree, _) = parser::parse(&source)?;

            match output {
                Some(p) => formatter::format(&tree, &mut fs::File::create(p)?),
                None => formatter::format(&tree, &mut io::stdout()),
            }?;
        }
        Command::Check { input } => {
            let start = std::time::Instant::now();
            let source = std::fs::read_to_string(&input)?;
            let (tree, spans) = parser::parse(&source)?;
            let mut spec = Specification::from(tree);
            spec.diagnose();
            let issues = issue::issues(&spec);

            let elapsed = start.elapsed();
            let mut count = 0;

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
