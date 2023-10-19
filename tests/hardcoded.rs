use wgsl_minifier::*;

fn minify(input_shader: &str) -> String {
    let mut module = naga::front::wgsl::parse_str(&input_shader).unwrap();

    // Now minify!
    remove_identifiers(&mut module);

    // Write to string
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    );
    let info = validator.validate(&module).unwrap();

    let output =
        naga::back::wgsl::write_string(&module, &info, naga::back::wgsl::WriterFlags::empty())
            .unwrap();

    // Minify string
    minify_wgsl_source_whitespace(&output)
}

#[test]
fn minify_1() {
    let src = "
    struct VertexOutput {
        @builtin(position) clip_position: vec4<f32>,
    };

    @vertex
    fn vs_main(
        @builtin(vertex_index) in_vertex_index: u32,
    ) -> VertexOutput {
        var out: VertexOutput;
        let x = f32(1 - i32(in_vertex_index)) * 0.5;
        let y = f32(i32(in_vertex_index & 1u) * 2 - 1) * 0.5;
        out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
        return out;
    }
    ";
    let expected = "struct a{@builtin(position)B:vec4<f32>}@vertex fn vs_main(@builtin(vertex_index)E:u32)->a{var d:a;d.B=vec4<f32>((f32((1-i32(E)))*0.5),(f32(((i32((E&1u))*2)-1))*0.5),0.0,1.0);return d;}";

    let got = minify(src);

    assert_eq!(expected, got);
}
