# WGSL Minifier
[![crates.io](https://img.shields.io/crates/v/wgsl-minifier.svg)](https://crates.io/crates/wgsl-minifier)
[![docs.rs](https://img.shields.io/docsrs/wgsl-minifier)](https://docs.rs/wgsl-minifier/latest/wgsl_minifier/)
[![crates.io](https://img.shields.io/crates/l/wgsl-minifier.svg)](https://github.com/LucentFlux/wgsl-minifier/blob/main/LICENSE)

A small tool built on top of [Naga](https://github.com/gfx-rs/naga) that makes WGSL shaders smaller by stripping names of non-exported functions and local variables, and removing as much whitespace as possible. 

# Usage
To minify your WGSL shader, simply run the following:

```
cargo install wgsl-minifier
wgsl-minifier path/to/your/shader.wgsl path/to/minified/output.wgsl
```
