use std::{cell::RefCell, num::NonZeroUsize, rc::Rc};

use anyhow::{Context, Result, bail};
use clap::Parser;
use mupdf::*;
use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, Event},
};

mod text;

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

    /// Do not emit copyable text
    #[arg(long)]
    no_text: bool,
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

    let writer = Rc::new(RefCell::new(Writer::new(Vec::new())));
    let mut reader = Reader::from_str(&svg);
    loop {
        match reader.read_event() {
            Ok(Event::Eof) => break,
            Ok(Event::End(e)) if e.name().as_ref() == b"svg" => {
                if !args.no_text {
                    writer
                        .borrow_mut()
                        .write_event(Event::Start(BytesStart::new("g")))?;
                    text::render(&page, writer.clone());
                    writer
                        .borrow_mut()
                        .write_event(Event::End(BytesEnd::new("g")))?;
                }
                writer.borrow_mut().write_event(Event::End(e))?;
            }
            Ok(e) => writer.borrow_mut().write_event(e.borrow())?,
            Err(e) => bail!("Error reading SVG: {}", e),
        }
    }
    let output = writer.replace(Writer::new(Vec::new())).into_inner();
    println!("{}", String::from_utf8_lossy(&output));

    Ok(())
}
