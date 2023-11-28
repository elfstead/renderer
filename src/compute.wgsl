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
    rot: mat3x3<f32>,
    aspect: f32,
    focal_length: f32,
    znear: f32,
    zfar: f32,
}

@group(2) @binding(0)
var<uniform> camera: Camera;

var<private> seed: f32;

// https://gist.github.com/munrocket/236ed5ba7e409b8bdf1ff6eca5dcdc39
fn rand() -> f32 {
    seed = sin(seed) * 43758.5453123;
    return fract(seed);
}

struct Collision {
    distance: f32,
    position: vec3f,
    normal: vec3f,
    color: vec3f,
}

const PI: f32 = 3.14159265358979323846264338327950288;

fn apply_lighting(pos: vec3<f32>, nor: vec3<f32>) -> vec3<f32> {
    var color = vec3f(0.0);
    var lights = 0;
    for (var i = 0; i < i32(arrayLength(&mesh_info)) - 1; i++) {
        let light_color = colors[i].ambient_color;
        if (light_color.r > 0.0 || light_color.g > 0.0 || light_color.b > 0.0) {
            let vertex_offset = mesh_info[i].vertex_offset;
            let index_offset = mesh_info[i].index_offset;
            let index_end = mesh_info[i+1].index_offset;
            for (var j: i32 = i32(index_offset); j < i32(index_end); j += 3) {
                let v0: vec3f = vertices[vertex_offset + indices[j]].pos;
                let v1: vec3f = vertices[vertex_offset + indices[j+1]].pos;
                let v2: vec3f = vertices[vertex_offset + indices[j+2]].pos;

                /*
                let a = length(v1-v0);
                let b = length(v2-v1);
                let c = length(v0-v2);

                let s = (a + b + c/2.0);
                let area = sqrt(s*(s-a)*(s-b)*(s-c)); 
                let importance = area*length(light_color);
                */

                // barycentric coordinates for homogenous probability over the surface
                // https://people.cs.kuleuven.be/~philip.dutre/GI/TotalCompendium.pdf
                let r1 = rand();
                let r2 = rand();
                let alpha = 1.0 - sqrt(r1);
                let beta = (1.0 - r2)*sqrt(r1);
                let gamma = r2*sqrt(r1);
                let point = alpha*v0 + beta*v1 + gamma*v2;

                let dir = point - pos;

                let inters = closest_intersection(pos, dir);

// I THINK THIS BLOCK IS WHERE IT GETS FUCKED!
                if (inters.distance >= length(dir) - 100.0) { //magic param, cant imagine it needs to be large ever except for at very very sharp angles, which can still be ignored, so just like 0.001 max for this right???
                    var add = light_color * 10000.0 * max(dot(nor,normalize(dir)), 0.0); //magic param, why am i multiplying by SO MUCH. Something must be wrong
                    //var add = light_color * 10000.0; //magic param
                    /*if (add.r < 0.1 && add.g < 0.1) {
                        return vec3f(1.0, 0.0, 0.0);
                    }*/
                    add /= (4.0*pow(length(dir), 2.0));
                    color += add; //I think color sometimes ends up negative and THAT SHOULDNT BE POSSIBLE
                    //color += vec3f(0.01);
                    lights += 1;
                }
            }
        }
    }
    //warning: possible div-by-0
    return color/f32(lights);
}

fn closest_intersection(ro: vec3<f32>, rd: vec3<f32>) -> Collision {
    var color: vec3f = vec3f(0f, 0f, 0f);
    let max_dist = 1e20f;
    var distance: f32 = max_dist;
    var position: vec3f = ro;
    var normal: vec3f = rd;
    for (var i = 0; i < i32(arrayLength(&mesh_info)) - 1; i++) {
        let vertex_offset = mesh_info[i].vertex_offset;
        let index_offset = mesh_info[i].index_offset;
        let index_end = mesh_info[i+1].index_offset;
        for (var j: i32 = i32(index_offset); j < i32(index_end); j += 3) {
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

            if (u >= 0.0 && v >= 0.0 && u + v <= 1.0 && t > 0.001) { //magic param
                if (distance > length(t*rd)) {
                    distance = length(t*rd);
                    color = colors[i].diffuse_color;
                    position = ro + t*rd;
                    normal = normalize(cross(e1, e2));
                }
            }
        }
    }
    //if (colors[0].diffuse_color.r == 1.0) {
        //color = colors[8].diffuse_color;
    //}
    //color = vec3f(rand(), rand(), rand());
    if (distance >= max_dist) {
        distance = -1.0;
    }
    var out: Collision;
    out.distance = distance;
    out.position = position;
    out.normal = normal;
    out.color = color;

    return out;
}

// https://iquilezles.org/articles/simplepathtracing/
fn trace_path(ro0: vec3<f32>, rd0: vec3<f32>) -> vec4f {
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
        surface_color *= col.color;
        color += surface_color * light;
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
fn main(@builtin(global_invocation_id) param: vec3<u32>, @builtin(num_workgroups) num: vec3<u32>) {
    seed = f32(pt_info.samples_per_pixel) * sin(dot(vec2<f32>(param.xy), vec2<f32>(12.9898, 4.1414))) * 43758.5453;
    let ident = mat3x3f(vec3f(1.0, 0.0, 0.0), vec3f(0.0, 1.0, 0.0), vec3f(0.0, 0.0, 1.0));
    var rd = ident * vec3f(
        f32(num.x - param.x) - f32(num.x)/2f,
        f32(num.y - param.y) - f32(num.y)/2f,
        f32(num.y)/2f
        );


    //rd = normalize(rd);

    let ro = camera.position;
    var color = trace_path(ro, rd);
    //color = clamp(color, vec4f(0.0), vec4f(1.0));
    color = max(vec4f(0.0), color);
    
    //pt[param.x + param.y*pt_info.width] = color;
    pt[param.x + param.y*pt_info.width] += color;
}
