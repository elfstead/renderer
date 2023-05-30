use std::path::Path;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    ambient_color: [f32; 3],
    diffuse_color: [f32, 3],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct ComputeInfo {
    num_vertices: u32,
    num_indices: u32,
}

fn layout_entries() -> Vec<wgpu::BindGroupLayoutEntry> {
    vec![
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
        // ComputeInfo
        wgpu::BindGroupLayoutEntry {
            binding: 2,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        },
    ]
}

fn bind_group_entries(
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    color_buffer: wgpu::Buffer,
    info_buffer: wgpu::Buffer,
) -> Vec<wgpu::BindGroupEntry> {
    vec![
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
        // ComputeInfo
        wgpu::BindGroupEntry {
            binding: 2,
            resource: info_buffer.as_entire_binding(),
        },
    ]
}

/*
* load model from .obj file
* a model can contain several meshes
* a mesh will be one or more connected triangle faces
* we step through one mesh at a time
* we assume the mesh uses a material (mtl file)
* we assume we are not using textures, only uniformly colored meshes
* we use only the ambient and diffuse color of the mesh
* the ambient color is emitted light
* the diffuse color is the 100% diffusely reflected color of the mesh
* we attach the color info to each vertex (slightly space infefficient)
* we concatenate all the vertices of all the meshes
* we combine all the index vectors as well, here we need to add offsets
* we create one vertex and one index buffer for the whole obj file
* we lastly create a computeinfo buffer with the length of the vertex and index buffers
* we return the bindgroup of the 3 buffers so that it can be used for whatever
*/
fn load<P: AsRef<Path>>(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    path: P,
) -> Result<wgpu::BindGroup, tobj::LoadError> {
    let (obj_models, obj_materials) = tobj::load_obj(path.as_ref(), &tobj::GPU_LOAD_OPTIONS)?;
    let obj_materials = obj_materials?;
    let vertices = obj_models
        .iter()
        .map(|m| {
            let mat_id = m.mesh
                .material_id
                .expect("mesh missing material id");
            (0..m.mesh.positions.len() / 3).into_iter()
            .map(|i| {
                Vertex {
                    position: [
                        m.mesh.positions[i * 3],
                        m.mesh.positions[i * 3 + 1],
                        m.mesh.positions[i * 3 + 2],
                    ],
                    ambient_color: obj_materials[mat_id].ambient.expect("mtl missing ambient color"),
                    diffuse_color: obj_materials[mat_id].diffuse,
                }
            })
        }).flatten().collect::<Vec<Vertex>>();

    let indices = obj_models
        .iter()
        .map(|m| {
            m.mesh.indices
        }).flatten().collect::<Vec<u32>>();
        
    
    for m in obj_models {
        let mesh = m.mesh;
        let mat_id = mesh
            .material_id
            .expect("Mesh does not have a material!!! Unrenderable");
        let amb_col = obj_materials[mat_id].ambient;
        let dif_col = obj_materials[mat_id].diffuse;
    }
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout,
        entries: bind_group_entries(),
    })
}
