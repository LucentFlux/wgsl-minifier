use wgsl_minifier::*;

fn minify(input_shader: &str) -> String {
    let mut module = naga::front::wgsl::parse_str(&input_shader).unwrap();

    // Now minify!
    minify_module(&mut module);

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
    minify_wgsl_source(&output)
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
    let expected = "var<private>d:u32;var<private>E:vec4<f32>=vec4<f32>(0.0,0.0,0.0,1.0);fn e(){let _e8=d;E=vec4<f32>((f32(1-bitcast<i32>(_e8))*0.5),(f32((bitcast<i32>(_e8&1u)*2)- 1)*0.5),0.0,1.0);E[1u]=-(E[1u]);return;}@vertex fn vs_main(@builtin(vertex_index)f:u32)->@builtin(position)vec4<f32>{d=f;e();E.y=-(E.y);return E;}";

    let got = minify(src);

    assert_eq!(expected, got);
}
