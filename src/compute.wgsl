@group(0) @binding(0)
var texture: texture_storage_2d<f32, write>;

fn closest_intersection(ro: vec3<f32>, rd: vec3<f32>) {
}

@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) param: vec3<u32>) {
    let coords = vec2(global_invocation_id.x, global_invocation_id.y);
    if (coords.x < 500) {
        textureStore(texture, coords, vec4<f32>(1.0, 1.0, 1.0, 1.0));
    }
}
