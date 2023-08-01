# WGSL Minifier
A small tool built on top of [Naga](https://github.com/gfx-rs/naga) that makes WGSL shaders smaller. 

# Usage
To minify your WGSL shader, simply run the following:

```
cargo install wgsl-minifier
cargo wgsl-minifier path/to/your/shader.wgsl path/to/minified/output.wgsl
```