use cgmath::*;
use std::path::Path;

use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    _padding: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct MeshInfo {
    vertex_offset: u32,
    index_offset: u32,
    ambient_color: [f32; 3],
    _padding: u32,
    diffuse_color: [f32; 3],
    _padding2: [u32; 3],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct ComputeInfo {
    num_meshes: u32,
}

pub fn bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("compute_bind_group_layout"),
        entries: &[
            // Vertices
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // Indices
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // MeshInfo
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // ComputeInfo
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    })
}

/*
* load model from .obj file
* a model can contain several meshes
* a mesh will be one or more connected triangle faces
* we assume the mesh uses a material (mtl file)
* we assume we are not using textures, only uniformly colored meshes
* we use only the ambient and diffuse color of the mesh
* the ambient color is emitted light
* the diffuse color is the 100% diffusely reflected color of the mesh
* we create one vertex and one index buffer for the whole obj file
* we create a meshinfo buffer with the offsets for each section of the vertex and index buffers
* the meshinfo buffer also contains the color info for each mesh
* we lastly create a computeinfo buffer with the length of the meshinfo buffer
* we return the bindgroup of the 4 buffers so that it can be used for whatever
*/
pub fn load<P: AsRef<Path>>(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    path: P,
) -> Result<wgpu::BindGroup, tobj::LoadError> {
    let (obj_models, obj_materials) = tobj::load_obj(path.as_ref(), &tobj::GPU_LOAD_OPTIONS)?;
    let obj_materials = obj_materials?;
    let vertices = obj_models
        .iter()
        .map(|m| {
            (0..m.mesh.positions.len() / 3).into_iter().map(|i| Vertex {
                position: [
                    m.mesh.positions[i * 3],
                    m.mesh.positions[i * 3 + 1],
                    m.mesh.positions[i * 3 + 2],
                ],
                _padding: 0,
            })
        })
        .flatten()
        .collect::<Vec<Vertex>>();

    let mut mesh_info = Vec::new();
    // could do this as well functionally with something like scan()
    let mut vertex_offset = 0;
    let mut index_offset = 0;
    for m in &obj_models {
        let mat_id = m.mesh.material_id.expect("mesh missing material id");
        let ccc = obj_materials[mat_id].diffuse.unwrap();
        println!(
            "{} {} MAT ID {} {} {}",
            m.name, mat_id, ccc[0], ccc[1], ccc[2]
        );
        mesh_info.push(MeshInfo {
            vertex_offset,
            index_offset,
            ambient_color: obj_materials[mat_id]
                .ambient
                .expect("mtl missing ambient color"),
            diffuse_color: obj_materials[mat_id]
                .diffuse
                .expect("mtl missing diffuse color"),
            _padding: 0,
            _padding2: [0, 0, 0],
        });
        vertex_offset += <usize as TryInto<u32>>::try_into(m.mesh.positions.len() / 3)
            .expect("too many vertices");
        index_offset +=
            <usize as TryInto<u32>>::try_into(m.mesh.indices.len()).expect("too many indices");
    }

    mesh_info.push(MeshInfo {
        vertex_offset,
        index_offset,
        ambient_color: [0.0, 0.0, 0.0],
        diffuse_color: [0.0, 0.0, 0.0],
        _padding: 0,
        _padding2: [0, 0, 0],
    });

    for m in &mesh_info {
        let ccc = m.diffuse_color;
        println!("{} {} {}", ccc[0], ccc[1], ccc[2]);
    }

    let indices = obj_models
        .iter()
        .map(|m| m.mesh.indices.clone()) // should avoid unnecessary clone
        .flatten()
        .collect::<Vec<u32>>();

    const SCREEN_W: usize = 25;
    const SCREEN_H: usize = 25;
    let mut screendraw = [[vec3(0.0, 0.0, 0.0); SCREEN_H]; SCREEN_W];

    let cam_r: Matrix3<f32> = cgmath::Matrix3::identity();
    let ro = cgmath::vec3(250.0, 250.0, -500.0);

    for screen_y in 0..SCREEN_H {
        for screen_x in 0..SCREEN_W {
            let mut distance: f32 = 1e20;
            let mut color = vec3(0.0, 0.0, 0.0);
            let px_vec: Vector3<f32> = cgmath::vec3(
                screen_x as f32 - SCREEN_W as f32 / 2.0,
                screen_y as f32 - SCREEN_H as f32 / 2.0,
                SCREEN_H as f32 / 2.0,
            );

            let rd =
                <cgmath::Matrix3<f32> as cgmath::Transform<cgmath::Point3<f32>>>::transform_vector(
                    &cam_r, px_vec,
                );

            for i in 0..(mesh_info.len() - 1) {
                let v_offs = mesh_info[i].vertex_offset;
                let i_offs = mesh_info[i].index_offset;
                let i_end = mesh_info[i + 1].index_offset;
                for j in i_offs / 3..i_end / 3 {
                    let v0: cgmath::Vector3<f32> = vertices
                        [v_offs as usize + indices[j as usize * 3] as usize]
                        .position
                        .into();
                    let v1: cgmath::Vector3<f32> = vertices
                        [v_offs as usize + indices[j as usize * 3 + 1] as usize]
                        .position
                        .into();
                    let v2: cgmath::Vector3<f32> = vertices
                        [v_offs as usize + indices[j as usize * 3 + 2] as usize]
                        .position
                        .into();

                    /*
                    println!(
                        "index: {}, vertex: {} {} {}, vo: {}",
                        indices[j as usize * 3],
                        v0.x,
                        v0.y,
                        v0.z,
                        v_offs
                    );
                    */

                    let e1 = v1 - v0;
                    let e2 = v2 - v0;
                    let b = ro - v0;

                    let n = cgmath::Vector3::cross(e1, e2);
                    let q = cgmath::Vector3::cross(b, rd);

                    let d = 1.0 / cgmath::dot(rd, n);
                    let u = d * dot(-q, e2);
                    let v = d * dot(q, e1);
                    let t = d * dot(-n, b);

                    if u >= 0.0 && v >= 0.0 && u + v <= 1.0 && t > 0.00001 {
                        if distance > cgmath::Vector3::magnitude(t * rd) {
                            distance = cgmath::Vector3::magnitude(t * rd);
                            color = vec3(1.0, 1.0, 1.0);
                        }
                    }
                }
            }
            screendraw[screen_x][screen_y] = color;
            print!("({}, {}, {})", color[0], color[1], color[2]);
        }
        println!();
    }

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::STORAGE,
    });

    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Index Buffer"),
        contents: bytemuck::cast_slice(&indices),
        usage: wgpu::BufferUsages::STORAGE,
    });

    let mesh_info_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Mesh Info Buffer"),
        contents: bytemuck::cast_slice(&mesh_info),
        usage: wgpu::BufferUsages::STORAGE,
    });

    let compute_info = ComputeInfo {
        // -1 because the last mesh is a dummy to show where we end
        num_meshes: (mesh_info.len() - 1).try_into().expect("too many meshes"),
    };

    let compute_info_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Compute Info Buffer"),
        contents: bytemuck::cast_slice(&[compute_info]),
        usage: wgpu::BufferUsages::UNIFORM,
    });

    Ok(device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout,
        entries: &[
            // Vertices
            wgpu::BindGroupEntry {
                binding: 0,
                resource: vertex_buffer.as_entire_binding(),
            },
            // Indices
            wgpu::BindGroupEntry {
                binding: 1,
                resource: index_buffer.as_entire_binding(),
            },
            // MeshInfo
            wgpu::BindGroupEntry {
                binding: 2,
                resource: mesh_info_buffer.as_entire_binding(),
            },
            // ComputeInfo
            wgpu::BindGroupEntry {
                binding: 3,
                resource: compute_info_buffer.as_entire_binding(),
            },
        ],
        label: Some("compute_bind_group"),
    }))
}
