struct PtInfo {
    width: u32,
    height: u32,
    samples_per_pixel: u32,
}

@group(0) @binding(0)
var<storage, read_write> pt: array<vec4<f32>>;
@group(0) @binding(1)
var<uniform> pt_info: PtInfo;

fn closest_intersection(ro: vec3<f32>, rd: vec3<f32>) {
}

@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) param: vec3<u32>) {
    let coords = vec3<u32>(param.x, param.y, param.z);
    if (coords.x < u32(500)) {
        pt[coords.x + coords.y*pt_info.width] = vec4<f32>(1.0, 1.0, 1.0, 1.0);
    } else if (coords.y > u32(300)) {
        pt[coords.x + coords.y*pt_info.width] = vec4<f32>(1.0, 0.2, 0.8, 1.0);
    } else {
        pt[coords.x + coords.y*pt_info.width] = vec4<f32>(0.1, 1.0, 0.0, 1.0);
    }
}
