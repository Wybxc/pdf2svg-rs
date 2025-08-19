use std::num::NonZeroUsize;

use anyhow::{Context, Result};
use clap::Parser;
use mupdf::*;

/// Convert PDF pages to SVG format
///
/// This tool converts individual pages from PDF files to SVG format using the MuPDF library.
/// The output SVG is printed to stdout, allowing for easy redirection to files or piping
/// to other tools.
///
/// Examples:
///
/// $ pdf2svg document.pdf > page1.svg
///
/// $ pdf2svg document.pdf --page 3 > page3.svg
#[derive(Parser)]
#[command(author, version)]
struct Args {
    /// The path to the PDF file to convert.
    file: String,

    /// The page number to convert.
    #[arg(short, long, default_value = "1")]
    page: NonZeroUsize,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let document = Document::open(&args.file).with_context(|| "Failed to open document")?;

    let page = document
        .pages()
        .with_context(|| "Failed to get pages")?
        .nth(usize::from(args.page) - 1)
        .with_context(|| format!("Page {} not found", args.page))?
        .with_context(|| format!("Failed to read page {}", args.page))?;
    let svg = page
        .to_svg(&Matrix::IDENTITY)
        .with_context(|| format!("Failed to render page {}", args.page))?;
    println!("{}", svg);

    Ok(())
}
