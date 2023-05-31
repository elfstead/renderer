@group(0) @binding(0)
var texture: texture_3d<f32>;

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
fn fs_main(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let coords = vec3<u32>(u32(pos.x), u32(pos.y), u32(0));
    //copy color from compute shader path trace texture buffer
    let color = textureLoad(texture, coords, 0);
    return color;
}
