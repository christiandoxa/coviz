use std::{
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, bail};
use clap::{Parser, Subcommand, ValueEnum};
use coviz::{Language, analyze_path, render_dot, render_html, render_json};

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
    let analysis = analyze_path(&args.input, args.language.into_analysis_language())
        .with_context(|| format!("failed to analyze {}", args.input.display()))?;

    let workspace = create_quick_workspace()?;
    let dot = render_dot(&analysis);
    fs::write(workspace.join("index.html"), render_html(&analysis))
        .with_context(|| format!("failed to write quick viewer in {}", workspace.display()))?;
    fs::write(workspace.join("graph.json"), render_json(&analysis)?)
        .with_context(|| format!("failed to write graph.json in {}", workspace.display()))?;
    fs::write(workspace.join("graph.dot"), &dot)
        .with_context(|| format!("failed to write graph.dot in {}", workspace.display()))?;
    if let Err(error) = render_quick_svg(&workspace) {
        eprintln!("failed to render Graphviz SVG, using browser fallback: {error}");
    }

    let listener = TcpListener::bind(("127.0.0.1", args.port))
        .with_context(|| format!("failed to bind localhost port {}", args.port))?;
    let url = format!("http://localhost:{}/", listener.local_addr()?.port());

    println!("coviz quick workspace: {}", workspace.display());
    println!("coviz quick viewer: {url}");

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
