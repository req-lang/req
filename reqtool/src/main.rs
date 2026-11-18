use std::{fs, io, path::PathBuf};

use clap::Parser;
use colored::Colorize;
use reqlang::{
    Specification, exporter,
    issue::{self, IssueCode, Kind, Severity, code::codes},
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Command::Fmt { input, output } => {
            let source = std::fs::read_to_string(&input)?;
            let (tree, info) = parser::parse(&source);
            let errors = info.errors;

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
        Command::Export { input, output } => {
            let source = std::fs::read_to_string(&input)?;
            let (tree, info) = parser::parse(&source);
            let errors = info.errors;
            report_syntax_errors(&source, &errors);

            // What the parser recovered is exported, so a specification being written still shows
            let spec = Specification::from(tree);
            match output {
                Some(p) => exporter::html::render(&spec, &mut fs::File::create(p)?),
                None => exporter::html::render(&spec, &mut io::stdout()),
            }?;

            if !errors.is_empty() {
                std::process::exit(1);
            }
        }
        Command::Check { input, json } => {
            let start = std::time::Instant::now();
            let source = std::fs::read_to_string(&input)?;
            let (tree, info) = parser::parse(&source);
            let errors = &info.errors;
            if !json {
                report_syntax_errors(&source, errors);
            }
            let mut spec = Specification::from(tree);
            spec.diagnose();
            let issues = issue::issues(&spec);

            let elapsed = start.elapsed();
            let mut count = errors.len();
            let mut json_paths = Vec::new();

            for ((idx, issues), path) in issues
                .indices()
                .zip(issues)
                .into_iter()
                .zip(spec.analysis.paths)
            {
                count += issues.len();
                if issues.is_empty() {
                    continue;
                }

                if !json {
                    eprintln!("{}", path.path.to_string().bold());
                }

                let mut json_issues = Vec::new();

                for issue in issues.into_iter() {
                    let span = issue.span(idx, &info);
                    let snippet = &source[span.range()];
                    let issue = Issue::from(issue);
                    if json {
                        json_issues.push(serde_json::json!({
                            "code": issue.code.to_string(),
                            "kind": issue.kind.as_str(),
                            "severity": issue.severity_name(),
                            "message": issue.message,
                            "span": span_json(&span),
                            "snippet": snippet,
                        }));
                    } else {
                        let sp = format!("{}", span.start).on_black();
                        eprintln!(
                            "{:12} {} {}\n{}",
                            sp,
                            issue.code.to_string().dimmed(),
                            issue.colored_message(),
                            snippet
                        );
                    }
                }

                if json {
                    json_paths.push(serde_json::json!({
                        "path": path.path.to_string(),
                        "issues": json_issues,
                    }));
                }
            }

            if json {
                let syntax_errors: Vec<_> = errors
                    .iter()
                    .map(|span| {
                        let end = span.end.index.min(source.len());
                        let start = span.start.index.min(end);
                        serde_json::json!({
                            "code": codes::SYNTAX_ERROR.to_string(),
                            "kind": Kind::Syntax.as_str(),
                            "message": SYNTAX_ERROR_MESSAGE,
                            "span": span_json(span),
                            "snippet": &source[start..end],
                        })
                    })
                    .collect();
                let report = serde_json::json!({
                    "syntax_errors": syntax_errors,
                    "paths": json_paths,
                    "issue_count": count,
                    "elapsed_ms": elapsed.as_secs_f64() * 1000.0,
                });
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                match count {
                    0 => println!("no issues ({:.2?})", elapsed),
                    c => eprintln!("found {} issue(s) ({:.2?})", c, elapsed),
                }
            }
        }
    }

    Ok(())
}

const SYNTAX_ERROR_MESSAGE: &str = "syntax error";

fn report_syntax_errors(source: &str, errors: &[Span]) {
    for span in errors {
        let end = span.end.index.min(source.len());
        let start = span.start.index.min(end);
        let snippet = source[start..end].bold();
        eprintln!(
            "{:12} {} {}",
            format!("{}", span.start).bold(),
            codes::SYNTAX_ERROR.to_string().dimmed(),
            SYNTAX_ERROR_MESSAGE.red()
        );
        if !snippet.is_empty() {
            eprintln!("    {}", snippet);
        }
    }
}

fn span_json(span: &Span) -> serde_json::Value {
    serde_json::json!({
        "start": { "line": span.start.line, "column": span.start.column, "index": span.start.index },
        "end": { "line": span.end.line, "column": span.end.column, "index": span.end.index },
    })
}

struct Issue {
    code: IssueCode,
    kind: Kind,
    message: String,
    severity: Severity,
}

impl Issue {
    pub fn severity_name(&self) -> &'static str {
        match self.severity {
            Severity::Critical => "critical",
            Severity::Severe => "severe",
            Severity::Moderate => "moderate",
            Severity::Light => "light",
        }
    }

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
            code: value.code(),
            kind: value.kind(),
            message: value.to_string(),
            severity: value.severity(),
        }
    }
}
