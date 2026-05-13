use std::{fs, path::PathBuf};

use anyhow::Context;
use clap::{Parser, ValueEnum};
use coviz::{Language, analyze_path, render_dot, render_json};

#[derive(Debug, Parser)]
#[command(
    name = "coviz",
    version,
    about = "Visualize source code logic as a call graph.",
    long_about = "coviz analyzes Go and Rust source files and emits a simple call graph in DOT or JSON format."
)]
struct Cli {
    /// File or directory to analyze.
    #[arg(default_value = ".")]
    input: PathBuf,

    /// Source language. Use auto to infer from file extensions.
    #[arg(short, long, value_enum, default_value_t = CliLanguage::Auto)]
    language: CliLanguage,

    /// Output format.
    #[arg(short, long, value_enum, default_value_t = OutputFormat::Dot)]
    format: OutputFormat,

    /// Output file. Omit or use "-" for stdout.
    #[arg(short, long)]
    output: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CliLanguage {
    Auto,
    Go,
    Rust,
}

impl CliLanguage {
    fn into_analysis_language(self) -> Option<Language> {
        match self {
            Self::Auto => None,
            Self::Go => Some(Language::Go),
            Self::Rust => Some(Language::Rust),
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum OutputFormat {
    Dot,
    Json,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let analysis = analyze_path(&cli.input, cli.language.into_analysis_language())
        .with_context(|| format!("failed to analyze {}", cli.input.display()))?;

    let output = match cli.format {
        OutputFormat::Dot => render_dot(&analysis),
        OutputFormat::Json => render_json(&analysis)?,
    };

    match cli.output.as_deref() {
        Some(path) if path.as_os_str() != "-" => {
            fs::write(path, output)
                .with_context(|| format!("failed to write {}", path.display()))?;
        }
        _ => println!("{output}"),
    }

    Ok(())
}
