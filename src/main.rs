use mupdf::*;

fn main() {
    let input = std::env::args().nth(1).unwrap();
    let document = Document::open(&input).unwrap();

    let page = document.pages().unwrap().next().unwrap().unwrap();
    let svg = page.to_svg(&Matrix::IDENTITY).unwrap();
    println!("{}", svg);
}
