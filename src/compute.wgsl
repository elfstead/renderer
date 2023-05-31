@group(0) @binding(0)
var texture: texture_storage_3d<rgba32float, write>;

fn closest_intersection(ro: vec3<f32>, rd: vec3<f32>) {
}

@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) param: vec3<u32>) {
    let coords = vec3<u32>(param.x, param.y, param.z);
    if (coords.x < u32(500)) {
        textureStore(texture, coords, vec4<f32>(1.0, 1.0, 1.0, 1.0));
    } else if (coords.y > u32(300)) {
        textureStore(texture, coords, vec4<f32>(1.0, 0.2, 0.8, 1.0));
    } else {
        textureStore(texture, coords, vec4<f32>(0.1, 1.0, 0.0, 1.0));
    }
}
