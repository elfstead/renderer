struct PtInfo {
    width: u32,
    height: u32,
    samples_per_pixel: u32,
}

@group(0) @binding(0)
var<storage, read_write> pt: array<vec4f>;
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
    num_lights: u32,
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
    rot: mat3x3f,
    aspect: f32,
    focal_length: f32,
    znear: f32,
    zfar: f32,
}

@group(2) @binding(0)
var<uniform> camera: Camera;

// The cornell scene is on the order of 500 units
const EPSILON: f32 = 0.001;

var<private> seed: u32;

// https://gist.github.com/munrocket/236ed5ba7e409b8bdf1ff6eca5dcdc39
// https://www.pcg-random.org/
fn pcg(n: u32) -> u32 {
    var h = n * 747796405u + 2891336453u;
    h = ((h >> ((h >> 28u) + 4u)) ^ h) * 277803737u;
    return (h >> 22u) ^ h;
}

fn rand() -> f32 {
    seed = pcg(seed);

    return f32(seed)/f32(0xffffffffu);
}

struct Collision {
    distance: f32,
    position: vec3f,
    normal: vec3f,
    color_idx: u32,
}

const PI: f32 = 3.14159265358979323846264338327950288;

fn apply_lighting(pos: vec3f, nor: vec3f) -> vec3f {
    var color = vec3f(0.0);
    var lights = 0;
    for (var i = 0; i < i32(arrayLength(&mesh_info)) - 1; i++) { // for each mesh
        let light_color = colors[i].ambient_color;
        if (light_color.r > 0.0 || light_color.g > 0.0 || light_color.b > 0.0) { // if it is a light
            let vertex_offset = mesh_info[i].vertex_offset;
            let index_offset = mesh_info[i].index_offset;
            let index_end = mesh_info[i+1].index_offset;
            for (var j: i32 = i32(index_offset); j < i32(index_end); j += 3) { // for every triangle in that light
                let v0: vec3f = vertices[vertex_offset + indices[j]].pos;
                let v1: vec3f = vertices[vertex_offset + indices[j+1]].pos;
                let v2: vec3f = vertices[vertex_offset + indices[j+2]].pos;

                // pick a random light point in the triangle

                // barycentric coordinates for homogenous probability over the surface
                // https://people.cs.kuleuven.be/~philip.dutre/GI/TotalCompendium.pdf
                let r1 = rand();
                let r2 = rand();
                let alpha = 1.0 - sqrt(r1);
                let beta = (1.0 - r2)*sqrt(r1);
                let gamma = r2*sqrt(r1);
                let point = alpha*v0 + beta*v1 + gamma*v2;

                // see if object point is illumineted by the light point
                let dir = point - pos;

                let inters = closest_intersection(pos, dir);

                // if object is illuminated
                if (inters.distance >= length(dir) - EPSILON) {
                    // calculate lighting
                    var add = light_color * 10000.0 * max(dot(nor,normalize(dir)), 0.0); //magic param, why am i multiplying by SO MUCH. Something must be wrong
                    add /= (4.0*pow(length(dir), 2.0));
                    color += add;
                    lights += 1;
                }
            }
        }
    }
    if (lights > 0) {
        return color/f32(lights);
    }
    return vec3f(0.0);
}

fn closest_intersection(ro: vec3f, rd: vec3f) -> Collision {
    var color_idx: i32 = 0;
    let max_dist = 1e20f;
    var distance: f32 = max_dist;
    var position: vec3f = ro;
    var normal: vec3f = rd;
    for (var i = 0; i < i32(arrayLength(&mesh_info)) - 1; i++) {
        let vertex_offset = mesh_info[i].vertex_offset;
        let index_offset = mesh_info[i].index_offset;
        let index_end = mesh_info[i+1].index_offset;
        for (var j = i32(index_offset); j < i32(index_end); j += 3) {
            // https://iquilezles.org/articles/intersectors/
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

            let dist2 = length(rd)*t;

            // if the intersection is within the triangle and not super close
            if (u >= 0.0 && v >= 0.0 && u + v <= 1.0 && dist2 > EPSILON) {
                if (dist2 < distance) {
                    distance = dist2;
                    color_idx = i;
                    position = ro + t*rd;
                    normal = normalize(cross(e1, e2));
                }
            }
        }
    }
    if (distance >= max_dist) {
        distance = -1.0;
    }
    var out: Collision;
    out.distance = distance;
    out.position = position;
    out.normal = normal;
    out.color_idx = u32(color_idx);

    return out;
}

// https://iquilezles.org/articles/simplepathtracing/
fn trace_path(ro0: vec3f, rd0: vec3f) -> vec4f {
    var color = vec3f(0.0);
    var surface_color = vec3f(1.0);
    var ro = ro0;
    var rd = rd0;
    for (var i = 0; i < 4; i++) {
        let col = closest_intersection(ro, rd);

        if (col.distance < 0.0) {
            if (i == 0) {
                color = vec3f(0.0, 0.1, 0.5); //bg/sky color
            }
            break;
        }
        
        let light = apply_lighting(col.position, col.normal);
        surface_color *= colors[col.color_idx].diffuse_color;
        color += surface_color * light; //does this make color end as more than 1?
        color += colors[col.color_idx].ambient_color;
        ro = col.position;
        rd = random_bounce(col.normal);
    }

    return vec4f(color, 1.0);
}


fn random_bounce(norm: vec3f) -> vec3f {
    return lambert(norm);
}

// https://web.archive.org/web/20170610002747/http://www.amietia.com/lambertnotangent.html
fn lambert(norm: vec3f) -> vec3f {
    let r1 = rand();
    let r2 = 2.0*rand() - 1.0;

    let theta = 2.0 * PI * r1;
    let sphere_point = vec3f(sqrt(1.0 - r2 * r2) * vec2f(cos(theta), sin(theta)), r2);
    return norm + sphere_point;
}

@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) param: vec3u, @builtin(num_workgroups) num: vec3u) {
    seed = pt_info.samples_per_pixel*param.x*param.y + param.x + param.y;
    let ident = mat3x3f(vec3f(1.0, 0.0, 0.0), vec3f(0.0, 1.0, 0.0), vec3f(0.0, 0.0, 1.0));
    var rd = ident * vec3f(
        f32(num.x - param.x) - f32(num.x)/2f + rand(),
        f32(num.y - param.y) - f32(num.y)/2f + rand(),
        f32(num.y)/2f
        );

    let ro = camera.position;
    var color = trace_path(ro, rd);
    color = clamp(color, vec4f(0.0), vec4f(1.0)); // i think clamping is a hack
    
    pt[param.x + param.y*pt_info.width] += color;
}
