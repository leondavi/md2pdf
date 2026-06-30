//! md2pdf — a self-contained Markdown to PDF converter (command-line).

use md2pdf::render;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Parser;

/// Convert a Markdown file to a nicely formatted PDF.
///
/// Fonts are embedded in the binary, so no external dependencies are required.
#[derive(Parser, Debug)]
#[command(name = "md2pdf", version, about, long_about = None)]
struct Cli {
    /// Input Markdown file (use "-" to read from stdin).
    input: String,

    /// Output PDF path. Defaults to the input name with a .pdf extension
    /// (or "out.pdf" when reading from stdin).
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Document title stored in the PDF metadata.
    #[arg(short, long)]
    title: Option<String>,

    /// Page size.
    #[arg(long, value_enum, default_value_t = render::PaperSize::A4)]
    paper: render::PaperSize,

    /// Page margin in millimetres.
    #[arg(long, default_value_t = 20.0)]
    margin: f64,

    /// Base font size in points.
    #[arg(long, default_value_t = 11)]
    font_size: u8,
}

fn run() -> Result<(), String> {
    let cli = Cli::parse();

    let markdown = if cli.input == "-" {
        use std::io::Read;
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .map_err(|e| format!("failed to read stdin: {e}"))?;
        buf
    } else {
        std::fs::read_to_string(&cli.input)
            .map_err(|e| format!("failed to read '{}': {e}", cli.input))?
    };

    let output = cli.output.unwrap_or_else(|| {
        if cli.input == "-" {
            PathBuf::from("out.pdf")
        } else {
            Path::new(&cli.input).with_extension("pdf")
        }
    });

    let title = cli.title.unwrap_or_else(|| {
        if cli.input == "-" {
            "Document".to_string()
        } else {
            Path::new(&cli.input)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Document")
                .to_string()
        }
    });

    let opts = render::RenderOptions {
        title,
        paper: cli.paper,
        margin_mm: cli.margin,
        base_font_size: cli.font_size,
    };

    render::markdown_to_pdf(&markdown, &output, &opts)
        .map_err(|e| format!("failed to render PDF: {e}"))?;

    println!("Wrote {}", output.display());
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("md2pdf: error: {e}");
            ExitCode::FAILURE
        }
    }
}
