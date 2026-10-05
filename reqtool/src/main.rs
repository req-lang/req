use std::{fs, io, path::PathBuf};

use clap::Parser;
use colored::Colorize;
use reqlang::{
    issue::{self, Severity, connection_errors, expr_errors, path_issues, reference_errors},
    node::entity::{Arena, Idx},
    renderer::formatter::entity,
    semantics::{
        connection::{
            Connect, Connected,
            entity::{Connection, Neighbors},
        },
        link::Resolve,
        path::{self, Locate, Located, Location},
        typing::Evaluate,
    },
    syntax::{
        location::Span,
        parser::{self},
    },
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
            let source = std::fs::read_to_string(&input)?;
            let arena = parser::parse(&source)?;
            let spans: Arena<_> = (&arena).into_iter().map(|n| n.payload.clone()).collect();

            let arena = arena.locate();
            let locations: Arena<_> = (&arena).into_iter().map(|n| n.payload.clone()).collect();

            let arena = arena.resolve().evaluate().connect();
            let arena: Arena<_> = arena
                .into_iter()
                .zip(locations)
                .zip(spans)
                .map(|((node, loc), span)| node.map(|conns| Payload::new(loc, span, conns)))
                .collect();

            let issues: Arena<_> = (&arena)
                .into_iter()
                .map(|node| {
                    path_issues(node)
                        .chain(reference_errors(node))
                        .chain(connection_errors(node))
                        .chain(expr_errors(node))
                        .map(Issue::from)
                        .collect::<Vec<_>>()
                })
                .collect();

            let elapsed = start.elapsed();
            let mut count = 0;

            for (node, issues) in arena.into_iter().zip(issues) {
                count += issues.len();
                if issues.is_empty() {
                    continue;
                }

                let path = node.payload.location.path;
                let span = node.payload.span;
                let sp = format!("{}", span.start).bold();
                let code = format!("{}", &source[span.start.index..span.end.index]).bold();
                eprintln!("{:12} {} ---> {}", sp, path, code);

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

struct Payload {
    location: Location,
    span: Span,
    neighbors: Neighbors<Connection<Idx>>,
}

impl Located for Payload {
    fn location(&self) -> &path::Location {
        &self.location
    }
}

impl Connected for Payload {
    fn neigbhors(&self) -> &Neighbors<Connection<Idx>> {
        &self.neighbors
    }
}

impl Payload {
    pub fn new(location: Location, span: Span, neighbors: Neighbors<Connection<Idx>>) -> Self {
        Self {
            location,
            span,
            neighbors,
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
