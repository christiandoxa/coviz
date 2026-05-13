use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, bail};
use clap::{Parser, Subcommand, ValueEnum};
use coviz::{
    Analysis, AnalysisOptions, Language, analyze_path, analyze_path_with_options, render_dot,
    render_html, render_json,
};
use rayon::prelude::*;
use serde::Serialize;

const GRAPHVIZ_SYNC_FUNCTION_LIMIT: usize = 350;
const GRAPHVIZ_SYNC_CALL_LIMIT: usize = 700;
const GRAPHVIZ_AUTO_FUNCTION_LIMIT: usize = 1_500;
const GRAPHVIZ_AUTO_CALL_LIMIT: usize = 3_000;

#[derive(Debug, Parser)]
#[command(
    name = "coviz",
    version,
    about = "Visualize source code logic as a call graph.",
    long_about = "coviz analyzes Go and Rust source files and emits a simple call graph in DOT or JSON format."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<CliCommand>,

    #[command(flatten)]
    graph: GraphArgs,
}

#[derive(Debug, Subcommand)]
enum CliCommand {
    /// Analyze source into a temporary browser viewer and open it.
    Quick(QuickArgs),
}

#[derive(Debug, Parser)]
struct GraphArgs {
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

#[derive(Debug, Parser)]
struct QuickArgs {
    /// File or directory to analyze.
    #[arg(default_value = ".")]
    input: PathBuf,

    /// Source language. Use auto to infer from file extensions.
    #[arg(short, long, value_enum, default_value_t = CliLanguage::Auto)]
    language: CliLanguage,

    /// Localhost port. Use 0 to pick an available port.
    #[arg(long, default_value_t = 0)]
    port: u16,

    /// Do not open the default browser.
    #[arg(long)]
    no_open: bool,

    /// Include test files and Rust #[cfg(test)] code.
    #[arg(long)]
    include_tests: bool,

    /// Graphviz SVG rendering strategy. Auto skips SVG for large graphs.
    #[arg(long, value_enum, default_value_t = QuickGraphviz::Auto)]
    graphviz: QuickGraphviz,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum QuickGraphviz {
    Auto,
    Always,
    Never,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GraphvizPlan {
    Sync,
    BackgroundSvg,
    BackgroundDotOnly,
    Skip,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(CliCommand::Quick(args)) => run_quick(args),
        None => run_graph(cli.graph),
    }
}

fn run_graph(args: GraphArgs) -> anyhow::Result<()> {
    let analysis = analyze_path(&args.input, args.language.into_analysis_language())
        .with_context(|| format!("failed to analyze {}", args.input.display()))?;

    let output = match args.format {
        OutputFormat::Dot => render_dot(&analysis),
        OutputFormat::Json => render_json(&analysis)?,
    };

    match args.output.as_deref() {
        Some(path) if path.as_os_str() != "-" => {
            fs::write(path, output)
                .with_context(|| format!("failed to write {}", path.display()))?;
        }
        _ => println!("{output}"),
    }

    Ok(())
}

fn run_quick(args: QuickArgs) -> anyhow::Result<()> {
    let started_at = Instant::now();
    let options = if args.include_tests {
        AnalysisOptions::default()
    } else {
        AnalysisOptions::without_tests()
    };
    eprintln!("coviz quick: analyzing {}", args.input.display());
    let analysis_started_at = Instant::now();
    let analysis =
        analyze_path_with_options(&args.input, args.language.into_analysis_language(), options)
            .with_context(|| format!("failed to analyze {}", args.input.display()))?;
    eprintln!(
        "coviz quick: analyzed {} functions / {} calls in {:.2}s",
        analysis.functions.len(),
        analysis.calls.len(),
        analysis_started_at.elapsed().as_secs_f32()
    );

    let workspace = create_quick_workspace()?;
    fs::write(workspace.join("index.html"), render_html(&analysis))
        .with_context(|| format!("failed to write quick viewer in {}", workspace.display()))?;
    fs::write(
        workspace.join("graph.json"),
        serde_json::to_string(&analysis)?,
    )
    .with_context(|| format!("failed to write graph.json in {}", workspace.display()))?;
    fs::write(
        workspace.join("source.json"),
        render_source_index(&analysis, &args.input)?,
    )
    .with_context(|| format!("failed to write source.json in {}", workspace.display()))?;

    let graphviz_plan = quick_graphviz_plan(args.graphviz, &analysis);
    match graphviz_plan {
        GraphvizPlan::Sync => {
            if let Err(error) = write_quick_dot_artifacts(&workspace, &analysis, true) {
                eprintln!("failed to render Graphviz SVG, using browser fallback: {error}");
            }
        }
        GraphvizPlan::BackgroundSvg => {
            spawn_quick_dot_task(workspace.clone(), analysis.clone(), true);
        }
        GraphvizPlan::BackgroundDotOnly => {
            eprintln!(
                "coviz quick: graph is large, skipping Graphviz SVG in auto mode (use --graphviz always to force it)"
            );
            spawn_quick_dot_task(workspace.clone(), analysis.clone(), false);
        }
        GraphvizPlan::Skip => {
            eprintln!("coviz quick: Graphviz disabled");
        }
    }

    let listener = TcpListener::bind(("127.0.0.1", args.port))
        .with_context(|| format!("failed to bind localhost port {}", args.port))?;
    let url = format!("http://localhost:{}/", listener.local_addr()?.port());

    println!("coviz quick workspace: {}", workspace.display());
    println!("coviz quick viewer: {url}");
    eprintln!(
        "coviz quick: viewer ready in {:.2}s",
        started_at.elapsed().as_secs_f32()
    );

    if !args.no_open
        && let Err(error) = open_default_browser(&url)
    {
        eprintln!("failed to open default browser: {error}");
    }

    serve_quick_workspace(listener, &workspace)
}

fn create_quick_workspace() -> anyhow::Result<PathBuf> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before UNIX epoch")?
        .as_millis();
    let workspace = PathBuf::from("/tmp").join(format!("coviz-{now}-{}", std::process::id()));
    fs::create_dir_all(&workspace)
        .with_context(|| format!("failed to create {}", workspace.display()))?;
    Ok(workspace)
}

fn quick_graphviz_plan(mode: QuickGraphviz, analysis: &Analysis) -> GraphvizPlan {
    match mode {
        QuickGraphviz::Never => GraphvizPlan::Skip,
        QuickGraphviz::Always => GraphvizPlan::BackgroundSvg,
        QuickGraphviz::Auto
            if analysis.functions.len() <= GRAPHVIZ_SYNC_FUNCTION_LIMIT
                && analysis.calls.len() <= GRAPHVIZ_SYNC_CALL_LIMIT =>
        {
            GraphvizPlan::Sync
        }
        QuickGraphviz::Auto
            if analysis.functions.len() <= GRAPHVIZ_AUTO_FUNCTION_LIMIT
                && analysis.calls.len() <= GRAPHVIZ_AUTO_CALL_LIMIT =>
        {
            GraphvizPlan::BackgroundSvg
        }
        QuickGraphviz::Auto => GraphvizPlan::BackgroundDotOnly,
    }
}

fn spawn_quick_dot_task(workspace: PathBuf, analysis: Analysis, render_svg: bool) {
    thread::spawn(move || {
        if let Err(error) = write_quick_dot_artifacts(&workspace, &analysis, render_svg) {
            eprintln!("coviz quick: failed to write Graphviz artifacts: {error}");
        }
    });
}

#[derive(Debug, Serialize)]
struct SourceIndex {
    root: String,
    functions: Vec<FunctionSource>,
    files: Vec<FileSource>,
}

#[derive(Debug, Serialize)]
struct FunctionSource {
    id: String,
    file: String,
    line: usize,
    snippet_start: usize,
    lines: Vec<SourceLine>,
}

#[derive(Debug, Serialize)]
struct SourceLine {
    number: usize,
    text: String,
}

#[derive(Debug, Serialize)]
struct FileSource {
    file: String,
    absolute_path: String,
    lines: Vec<SourceLine>,
}

fn render_source_index(analysis: &Analysis, input: &Path) -> anyhow::Result<String> {
    let input = input
        .canonicalize()
        .with_context(|| format!("failed to resolve {}", input.display()))?;
    let root = if input.is_file() {
        input
            .parent()
            .unwrap_or_else(|| Path::new(""))
            .to_path_buf()
    } else {
        input
    };

    let unique_files: Vec<_> = analysis
        .functions
        .iter()
        .map(|function| function.file.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let file_sources: BTreeMap<String, FileSource> = unique_files
        .par_iter()
        .map(|file| {
            let path = root.join(file);
            let source = fs::read(&path)
                .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
                .unwrap_or_default();
            let lines = source
                .lines()
                .enumerate()
                .map(|(index, text)| SourceLine {
                    number: index + 1,
                    text: text.to_string(),
                })
                .collect();
            (
                file.clone(),
                FileSource {
                    file: file.clone(),
                    absolute_path: path.display().to_string(),
                    lines,
                },
            )
        })
        .collect::<Vec<_>>()
        .into_iter()
        .collect();

    let mut functions = Vec::with_capacity(analysis.functions.len());
    for function in &analysis.functions {
        let snippet_start = function.line.saturating_sub(5).max(1);
        let snippet_end = function.line + 5;
        let lines = file_sources
            .get(&function.file)
            .map(|file| {
                file.lines
                    .iter()
                    .filter(|line| (snippet_start..=snippet_end).contains(&line.number))
                    .map(|line| SourceLine {
                        number: line.number,
                        text: line.text.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        functions.push(FunctionSource {
            id: function.id.clone(),
            file: function.file.clone(),
            line: function.line,
            snippet_start,
            lines,
        });
    }

    Ok(serde_json::to_string(&SourceIndex {
        root: root.display().to_string(),
        functions,
        files: file_sources.into_values().collect(),
    })?)
}

fn write_quick_dot_artifacts(
    workspace: &Path,
    analysis: &Analysis,
    render_svg: bool,
) -> anyhow::Result<()> {
    let dot = render_dot(analysis);
    fs::write(workspace.join("graph.dot"), &dot)
        .with_context(|| format!("failed to write graph.dot in {}", workspace.display()))?;

    if render_svg {
        render_quick_svg(workspace)?;
    }

    Ok(())
}

fn render_quick_svg(workspace: &Path) -> anyhow::Result<()> {
    let output = Command::new("dot")
        .args(["-Tsvg", "graph.dot", "-o", "graph.svg"])
        .current_dir(workspace)
        .output()
        .context("failed to launch Graphviz dot")?;

    if !output.status.success() {
        bail!(
            "dot exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    Ok(())
}

fn open_default_browser(url: &str) -> anyhow::Result<()> {
    let status = if cfg!(target_os = "windows") {
        Command::new("cmd").args(["/C", "start", "", url]).status()
    } else if cfg!(target_os = "macos") {
        Command::new("open").arg(url).status()
    } else {
        Command::new("xdg-open").arg(url).status()
    }
    .context("failed to launch browser opener")?;

    if !status.success() {
        bail!("browser opener exited with {status}");
    }

    Ok(())
}

fn serve_quick_workspace(listener: TcpListener, workspace: &Path) -> anyhow::Result<()> {
    for stream in listener.incoming() {
        let stream = stream.context("failed to accept browser connection")?;
        handle_quick_request(stream, workspace)?;
    }

    Ok(())
}

fn handle_quick_request(mut stream: TcpStream, workspace: &Path) -> anyhow::Result<()> {
    let mut buffer = [0_u8; 2048];
    let read = stream
        .read(&mut buffer)
        .context("failed to read HTTP request")?;
    let request = String::from_utf8_lossy(&buffer[..read]);
    let request_line = request.lines().next().unwrap_or_default();
    let path = request_line
        .split_whitespace()
        .nth(1)
        .unwrap_or("/")
        .split('?')
        .next()
        .unwrap_or("/");

    let (status, content_type, body) = match path {
        "/" | "/index.html" => {
            file_response(workspace.join("index.html"), "text/html; charset=utf-8")?
        }
        "/graph.json" => file_response(
            workspace.join("graph.json"),
            "application/json; charset=utf-8",
        )?,
        "/source.json" => file_response(
            workspace.join("source.json"),
            "application/json; charset=utf-8",
        )?,
        "/graph.dot" => file_response(
            workspace.join("graph.dot"),
            "text/vnd.graphviz; charset=utf-8",
        )?,
        "/graph.svg" => file_response(workspace.join("graph.svg"), "image/svg+xml; charset=utf-8")
            .unwrap_or((
                "404 Not Found",
                "text/plain; charset=utf-8",
                Vec::from("graph.svg not found\n"),
            )),
        _ => (
            "404 Not Found",
            "text/plain; charset=utf-8",
            Vec::from("not found\n"),
        ),
    };

    write_http_response(&mut stream, status, content_type, &body)
}

fn file_response(
    path: PathBuf,
    content_type: &'static str,
) -> anyhow::Result<(&'static str, &'static str, Vec<u8>)> {
    let body = fs::read(&path).with_context(|| format!("failed to read {}", path.display()))?;
    Ok(("200 OK", content_type, body))
}

fn write_http_response(
    stream: &mut TcpStream,
    status: &str,
    content_type: &str,
    body: &[u8],
) -> anyhow::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .context("failed to write HTTP headers")?;
    stream.write_all(body).context("failed to write HTTP body")
}

#[cfg(test)]
mod tests {
    use coviz::Function;

    use super::{
        GRAPHVIZ_AUTO_FUNCTION_LIMIT, GRAPHVIZ_SYNC_FUNCTION_LIMIT, GraphvizPlan, QuickGraphviz,
        quick_graphviz_plan,
    };

    #[test]
    fn auto_graphviz_plan_skips_svg_for_large_graphs() {
        let analysis = coviz::Analysis {
            functions: (0..=GRAPHVIZ_AUTO_FUNCTION_LIMIT)
                .map(|index| Function {
                    id: format!("f{index}"),
                    name: format!("function_{index}"),
                    file: "src/lib.rs".to_string(),
                    line: index + 1,
                })
                .collect(),
            calls: Vec::new(),
        };

        assert_eq!(
            quick_graphviz_plan(QuickGraphviz::Auto, &analysis),
            GraphvizPlan::BackgroundDotOnly
        );
    }

    #[test]
    fn auto_graphviz_plan_keeps_small_graphs_synchronous() {
        let analysis = coviz::Analysis {
            functions: (0..GRAPHVIZ_SYNC_FUNCTION_LIMIT)
                .map(|index| Function {
                    id: format!("f{index}"),
                    name: format!("function_{index}"),
                    file: "src/lib.rs".to_string(),
                    line: index + 1,
                })
                .collect(),
            calls: Vec::new(),
        };

        assert_eq!(
            quick_graphviz_plan(QuickGraphviz::Auto, &analysis),
            GraphvizPlan::Sync
        );
    }
}
