struct PtInfo {
    width: u32,
    height: u32,
    samples_per_pixel: u32,
}

@group(0) @binding(0)
var<storage, read_write> pt: array<vec4<f32>>;
@group(0) @binding(1)
var<uniform> pt_info: PtInfo;

struct Vertex {
    pos: vec3f,
}

struct MeshInfo {
    vertex_offset: u32,
    index_offset: u32,
}

struct Colors {
    ambient_color: vec3f,
    diffuse_color: vec3f,
}

struct ComputeInfo {
    num_meshes: u32,
}

@group(1) @binding(0)
var<storage> vertices: array<Vertex>;
@group(1) @binding(1)
var<storage> indices: array<u32>;
@group(1) @binding(2)
var<storage> mesh_info: array<MeshInfo>;
@group(1) @binding(3)
var<storage> colors: array<Colors>;
@group(1) @binding(4)
var<uniform> compute_info: ComputeInfo;

struct Camera {
    position: vec3f,
    yaw: f32,
    pitch: f32,
    rot: mat3x3<f32>,
    aspect: f32,
    focal_length: f32,
    znear: f32,
    zfar: f32,
}

@group(2) @binding(0)
var<uniform> camera: Camera;

// https://iquilezles.org/articles/intersectors/
fn closest_intersection(ro: vec3<f32>, rd: vec3<f32>) -> vec4f {
    var color: vec3f = vec3f(0f, 0f, 0f);
    var distance: f32 = 1e20f;
    for (var i = 0; i < i32(arrayLength(&mesh_info)) - 1; i++) {
        let vertex_offset = mesh_info[i].vertex_offset;
        let index_offset = mesh_info[i].index_offset;
        let index_end = mesh_info[i+1].index_offset;
        for (var j: i32 = i32(index_offset); j < i32(index_end); j += 3) {
            let v0: vec3f = vertices[vertex_offset + indices[j]].pos;
            let v1: vec3f = vertices[vertex_offset + indices[j+1]].pos;
            let v2: vec3f = vertices[vertex_offset + indices[j+2]].pos;

            let e1 = v1 - v0;
            let e2 = v2 - v0;
            let b = ro - v0;

            let n = cross(e1, e2);
            let q = cross(b, rd);

            let d = 1.0/dot(rd, n);
            let u = d*dot(-q, e2);
            let v = d*dot(q, e1);
            let t = d*dot(-n, b);

            if (u >= 0.0 && v >= 0.0 && u + v <= 1.0 && t > 0.00001) {
                if (distance > length(t*rd)) {
                    distance = length(t*rd);
                    color = colors[i].diffuse_color;
                }
            }
        }
    }
    //if (colors[0].diffuse_color.r == 1.0) {
        //color = colors[8].diffuse_color;
    //}
    return vec4f(color, 1.0);
}

@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) param: vec3<u32>, @builtin(num_workgroups) num: vec3<u32>) {
    let ident = mat3x3f(vec3f(1.0, 0.0, 0.0), vec3f(0.0, 1.0, 0.0), vec3f(0.0, 0.0, 1.0));
    var rd = ident * vec3f(
        f32(num.x - param.x) - f32(num.x)/2f,
        f32(num.y - param.y) - f32(num.y)/2f,
        f32(num.y)/2f
        );


    //rd = normalize(rd);

    let ro = camera.position;
    let color = closest_intersection(ro, rd);
    
    pt[param.x + param.y*pt_info.width] = color;
}
