use wgsl_minifier::*;

use pretty_assertions::assert_str_eq;

fn minify(input_shader: &str) -> String {
    let mut module = naga::front::wgsl::parse_str(input_shader).unwrap();

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

    fn dead(v1: u32, v2: u32) -> u32 {
        return countOneBits(v1) + v2;
    }

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
    let expected = "struct a{@builtin(position)B:vec4<f32>}fn C(c:u32,D:u32)->u32{return(countOneBits(c)+D);}@vertex fn vs_main(@builtin(vertex_index)e:u32)->a{var E:a;E.B=vec4<f32>((f32(1i-i32(e))*0.5f),(f32((i32(e&1u)*2i)- 1i)*0.5f),0f,1f);let _e22=E;return _e22;}";

    let got = minify(src);

    assert_str_eq!(expected, got);
}

#[test]
fn minify_2() {
    let src = "
    struct OutputElement {
        x: u32,
        y: u32,
        z: u32,
    }
    struct Output {
        elements: array<OutputElement>,
    }

    @group(2) @binding(0) var<storage, read_write> output: Output;

    fn do_work(global_id: vec3<u32>) {
        output.elements[global_id.x - 1u] = OutputElement(global_id.x - 1u, global_id.x + 1u, global_id.x * 4u);
    }

    @compute
    @workgroup_size(256,1,1)
    fn comp_main(
        @builtin(global_invocation_id) global_id: vec3<u32>,
    ) {
        if global_id.x == 0u {
            return;
        }
        do_work(global_id);
    }
    ";
    let expected = "struct a{B:u32,b:u32,C:u32}struct D{d:array<a>}@group(2)@binding(0)var<storage,read_write>e:D;fn F(f:vec3<u32>){e.d[(f.x- 1u)]=a((f.x- 1u),(f.x+1u),(f.x*4u));return;}@compute@workgroup_size(256,1,1)fn comp_main(@builtin(global_invocation_id)g:vec3<u32>){if(g.x==0u){return;}F(g);return;}";

    let got = minify(src);

    assert_str_eq!(expected, got);
}
