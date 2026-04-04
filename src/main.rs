use std::{collections::HashMap, fs, io, path::PathBuf, process};

use clap::Parser;
use colored::Colorize;
use reqtool::{
    issue::{self, IssueIterator, Severity},
    node::{self},
    path::NodePathIter,
    renderer::formatter,
    semantics::{
        connection::{self},
        reference::{self},
        typing::{self},
    },
    syntax::{
        self,
        parser::{self},
    },
    utils::iter::NodeIter,
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

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Fmt { input, output } => {
            let source = std::fs::read_to_string(&input).expect("Failed to read input");

            let root = parser::parse(&source).unwrap_or_else(|err| {
                eprintln!("syntax error at {}", err.start);
                process::exit(1);
            });

            let nd = node::map(root, syntax::entity::Node::from);
            let result = match output {
                Some(p) => {
                    let fd = fs::File::create(p).unwrap_or_else(|err| {
                        eprintln!("unable to create the output file: {err}");
                        process::exit(1);
                    });
                    let mut writer = io::BufWriter::new(fd);
                    formatter::entity::format(nd.borrow(), 0, &mut writer)
                }
                None => formatter::entity::format(nd.borrow(), 0, &mut io::stdout()),
            };

            result.unwrap_or_else(|err| {
                eprintln!("failed to write output: {err}");
            });
        }
        Command::Check { input } => {
            let start = std::time::Instant::now();
            let source = fs::read_to_string(&input).unwrap_or_else(|err| {
                eprintln!("error reading {}: {err}", input.display());
                process::exit(1);
            });

            let root = parser::parse(&source).unwrap_or_else(|err| {
                eprintln!("syntax error at {}", err.start);
                process::exit(1);
            });

            let spans: HashMap<_, _> = NodePathIter::new(root.clone())
                .map(|(path, node)| (path, node.borrow().span))
                .collect();

            let ast = node::map(root, syntax::entity::Node::from);
            let resolved = reference::resolve(ast);
            let typed = typing::evaluate(resolved.clone());
            let connected = connection::connect(resolved.clone());
            let elapsed = start.elapsed();

            let mut count = 0;
            for (((path, conn), res), node) in NodePathIter::new(connected)
                .zip(NodeIter::new(resolved))
                .zip(NodeIter::new(typed))
            {
                let issues: Vec<_> = IssueIterator::from(node.borrow())
                    .map(Issue::from)
                    .chain(IssueIterator::from(conn.borrow()).map(Issue::from))
                    .chain(IssueIterator::from(res.borrow()).map(Issue::from))
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
