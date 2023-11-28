use crate::model;
use glam::Mat3;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Camera {
    position: [f32; 3],
    _padding: u32,
    yaw: f32,
    pitch: f32,
    rot: [[f32; 3]; 3],
    _padding2: [u32; 3],
    aspect: f32,
    focal_length: f32,
    znear: f32,
    zfar: f32,
    _padding3: [u32; 2],
}

pub struct Pt {
    pt_buffer: wgpu::Buffer,
    pt_info_buffer: wgpu::Buffer,
    pt_bind_group_layout: wgpu::BindGroupLayout,
    pt_bind_group: wgpu::BindGroup,
    compute_pipeline: wgpu::ComputePipeline,
    samples_per_pixel: u32,
    size: winit::dpi::PhysicalSize<u32>,
    model_bind_group: wgpu::BindGroup,
    camera_bind_group: wgpu::BindGroup,
}

impl Pt {
    pub fn new(device: &wgpu::Device, size: winit::dpi::PhysicalSize<u32>) -> Self {
        let model_bind_group_layout = model::bind_group_layout(&device);
        let model_bind_group =
            model::load(&device, &model_bind_group_layout, "res/cornell_box.obj").unwrap();

        let camera = Camera {
            position: [250.0, 250.0, -500.0],
            _padding: 0,
            yaw: 0.0,
            pitch: 0.0,
            rot: Mat3::IDENTITY.to_cols_array_2d(),
            _padding2: [0, 0, 0],
            aspect: size.width as f32 / size.height as f32,
            focal_length: size.height as f32 / 2.0,
            znear: 0.1,
            zfar: 100.0,
            _padding3: [0, 0],
        };

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera]),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("camera_bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera_bind_group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let samples_per_pixel = 0;

        /*
         * We need to create a special texture buffer to draw our result to
         * since we cannot draw directly to the screen from a compute shader
         */
        let (pt_buffer, pt_info_buffer) = create_pt_bufs(device, size, samples_per_pixel);

        /*
         * The fragment and compute shaders will both access the same pt texture
         * but one will do so as a sampled texture and the other as a storage texture
         * note that wgpu might unofficially support using only the pt_bind_group
         * for both purposes by setting access: ReadWrite or something, havent tried yet
         */

        let pt_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("pt_bind_group_layout"),
                entries: &[
                    // Pt
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // PtInfo
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let pt_bind_group =
            create_pt_bind_group(device, &pt_buffer, &pt_info_buffer, &pt_bind_group_layout);
        let compute_shader = device.create_shader_module(wgpu::include_wgsl!("compute.wgsl"));

        /*
         * we will have one bind group for the texture we are drawing to
         * and one bind group for the model
         */
        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Compute Pipeline Layout"),
                bind_group_layouts: &[
                    &pt_bind_group_layout,
                    &model_bind_group_layout,
                    &camera_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute Pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader,
            entry_point: "main",
        });

        Pt {
            pt_buffer,
            pt_info_buffer,
            pt_bind_group_layout,
            pt_bind_group,
            compute_pipeline,
            samples_per_pixel,
            size,
            model_bind_group,
            camera_bind_group,
        }
    }

    pub fn encode_compute(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Compute Pass"),
            timestamp_writes: None,
        });
        compute_pass.set_pipeline(&self.compute_pipeline);
        compute_pass.set_bind_group(0, &self.pt_bind_group, &[]);
        compute_pass.set_bind_group(1, &self.model_bind_group, &[]);
        compute_pass.set_bind_group(2, &self.camera_bind_group, &[]);

        compute_pass.dispatch_workgroups(self.size.width, self.size.height, 1);
    }

    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.pt_bind_group_layout
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.pt_bind_group
    }

    pub fn resize(&mut self, device: &wgpu::Device, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;
        (self.pt_buffer, self.pt_info_buffer) =
            create_pt_bufs(device, new_size, self.samples_per_pixel);
        self.pt_bind_group = create_pt_bind_group(
            device,
            &self.pt_buffer,
            &self.pt_info_buffer,
            &self.pt_bind_group_layout,
        );
    }

    pub fn next_frame(&mut self, queue: &wgpu::Queue) {
        self.samples_per_pixel += 1;
        let pt_info = PtInfo {
            width: self.size.width,
            height: self.size.height,
            samples_per_pixel: self.samples_per_pixel,
            _padding: 0,
        };
        queue.write_buffer(&self.pt_info_buffer, 0, bytemuck::cast_slice(&[pt_info]));
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct PtInfo {
    width: u32,
    height: u32,
    samples_per_pixel: u32,
    _padding: u32,
}

fn create_pt_bufs(
    device: &wgpu::Device,
    size: winit::dpi::PhysicalSize<u32>,
    samples_per_pixel: u32,
) -> (wgpu::Buffer, wgpu::Buffer) {
    let pt_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("pt_buffer"),
        size: (wgpu::TextureFormat::Rgba32Float.block_size(None).unwrap()
            * size.width
            * size.height)
            .into(),
        usage: wgpu::BufferUsages::STORAGE,
        mapped_at_creation: false,
    });
    let pt_info = PtInfo {
        width: size.width,
        height: size.height,
        samples_per_pixel,
        _padding: 0,
    };
    let pt_info_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Path Trace Info Buffer"),
        contents: bytemuck::cast_slice(&[pt_info]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });
    (pt_buffer, pt_info_buffer)
}

fn create_pt_bind_group(
    device: &wgpu::Device,
    pt_buffer: &wgpu::Buffer,
    pt_info_buffer: &wgpu::Buffer,
    pt_bg_layout: &wgpu::BindGroupLayout,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("pt_bind_group"),
        layout: pt_bg_layout,
        entries: &[
            // Pt
            wgpu::BindGroupEntry {
                binding: 0,
                resource: pt_buffer.as_entire_binding(),
            },
            // PtInfo
            wgpu::BindGroupEntry {
                binding: 1,
                resource: pt_info_buffer.as_entire_binding(),
            },
        ],
    })
}
