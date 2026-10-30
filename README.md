# Jlreq

An unreleased Rust library for Japanese text layout from UTF-8 text and font data.

## Installation

```sh
# 0.1.0 is unreleased; work from the source checkout.
git clone https://github.com/P4suta/jlreq.git
cd jlreq
mise install
```

## Usage

```rust
use jlreq::{FontLibrary, LayoutOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let font_path = std::env::args()
        .nth(1)
        .ok_or("pass a font file, e.g. NotoSansJP-Regular.otf")?;

    let mut fonts = FontLibrary::new();
    fonts.register_font(std::fs::read(font_path)?)?;

    let options = LayoutOptions::try_new(240.0, 16.0)?;
    let layout = jlreq::layout("日本語組版 — draw-ready glyphs", &fonts, options)?;

    println!("{}", layout.glyphs().count());
    Ok(())
}
```

## Documentation

[Read the documentation](docs/README.md).

## License

Licensed under the [Apache License, Version 2.0](LICENSE-APACHE) or the [Massachusetts Institute of Technology (MIT) license](LICENSE-MIT), at your option.
