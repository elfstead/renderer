@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32,
) -> @builtin(position) vec4<f32> {
    //cover screen with triangle
    //https://github.com/gfx-rs/wgpu/blob/trunk/wgpu/examples/skybox/shader.wgsl
    let tmp1 = i32(in_vertex_index) / 2;
    let tmp2 = i32(in_vertex_index) & 1;
    let pos = vec4<f32>(
        f32(tmp1) * 4.0 - 1.0,
        f32(tmp2) * 4.0 - 1.0,
        1.0,
        1.0
    );
    return pos;
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    //copy color from compute shader path trace texture buffer
    return vec4<f32>(0.3, 0.2, 0.1, 1.0);
}
