# WGSL Minifier
[![crates.io](https://img.shields.io/crates/v/wgsl-minifier.svg)](https://crates.io/crates/wgsl-minifier)
[![docs.rs](https://img.shields.io/docsrs/wgsl-minifier)](https://docs.rs/wgsl-minifier/latest/wgsl_minifier/)
[![crates.io](https://img.shields.io/crates/l/wgsl-minifier.svg)](https://github.com/LucentFlux/wgsl-minifier/blob/main/LICENSE)

A small tool built on top of [Naga](https://github.com/gfx-rs/naga) that makes WGSL shaders smaller by performing simple dead code elimination, stripping names of non-exported functions and local variables, and removing as much whitespace as possible. 

# Usage
To minify your WGSL shader, simply run the following:

```bash
cargo install --features 'bin' wgsl-minifier
wgsl-minifier path/to/your/shader.wgsl path/to/minified/output.wgsl
```

# As a library

To use this crate as a library, for example in a game engine or larger preprocessor, two calls must be made. The first strips identifiers to smaller ones where possible. The second removes unnecessary whitespace, commas, and parentheses in a source string:

```rust
let mut module = /* your source here, or */ naga::Module::default();

// Now minify!
wgsl_minifier::minify_module(&mut module);

// Write to WGSL string
let mut validator = naga::valid::Validator::new(
    naga::valid::ValidationFlags::all(),
    naga::valid::Capabilities::all(),
);
let info = validator.validate(&module).unwrap();
let output = naga::back::wgsl::write_string(&module, &info, naga::back::wgsl::WriterFlags::empty()).unwrap();

// Minify string
let output = wgsl_minifier::minify_wgsl_source(&output);
```