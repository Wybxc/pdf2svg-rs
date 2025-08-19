# pdf2svg

Convert PDF pages to SVG format using [MuPDF](https://mupdf.com) library.

## Installation

```bash
# Clone the repository
git clone <repository-url>
cd pdf2svg

# Build the project
cargo build --release

# Install globally (optional)
cargo install --path .
```

## Usage

```bash
pdf2svg <FILE> [OPTIONS]
```

### Arguments

- `<FILE>` - The path to the PDF file to convert

### Options

- `-p, --page <PAGE>` - The page number to convert (default: 1)
- `-h, --help` - Print help information
- `-V, --version` - Print version information

### Examples

```bash
# Convert first page to SVG and save to file
pdf2svg document.pdf > page1.svg

# Convert page 3 to SVG
pdf2svg document.pdf --page 3 > page3.svg

# Convert page 5 using short option
pdf2svg document.pdf -p 5 > page5.svg
```

## License

This project is licensed under GNU Affero General Public License v3.0.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Acknowledgments

- [MuPDF](https://mupdf.com) for the PDF and SVG rendering capabilities.
- [mupdf-rs](https://github.com/messense/mupdf-rs) for the Rust bindings of MuPDF.
