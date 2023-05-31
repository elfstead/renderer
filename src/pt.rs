use crate::model;

pub struct Pt {
    texture: wgpu::Texture,
    render_bind_group_layout: wgpu::BindGroupLayout,
    compute_bind_group_layout: wgpu::BindGroupLayout,
    render_bind_group: wgpu::BindGroup,
    compute_bind_group: wgpu::BindGroup,
    compute_pipeline: wgpu::ComputePipeline,
    samples_per_pixel: u32,
    size: winit::dpi::PhysicalSize<u32>,
    model_bind_group: wgpu::BindGroup,
}

impl Pt {
    pub fn new(device: &wgpu::Device, size: winit::dpi::PhysicalSize<u32>) -> Self {
        let model_bind_group_layout = model::bind_group_layout(&device);
        let model_bind_group =
            model::load(&device, &model_bind_group_layout, "res/cornell_box.obj").unwrap();

        let samples_per_pixel = 4;

        /*
         * We need to create a special texture buffer to draw our result to
         * since we cannot draw directly to the screen from a compute shader
         */
        let texture = create_texture(device, size, samples_per_pixel);

        /*
         * The fragment and compute shaders will both access the same pt texture
         * but one will do so as a sampled texture and the other as a storage texture
         * note that wgpu might unofficially support using only the pt_bind_group
         * for both purposes by setting access: ReadWrite or something, havent tried yet
         */

        let render_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("pt_render_bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D3,
                        multisampled: false,
                    },
                    count: None,
                }],
            });

        let compute_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("pt_compute_bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba32Float,
                        view_dimension: wgpu::TextureViewDimension::D3,
                    },
                    count: None,
                }],
            });

        let (render_bind_group, compute_bind_group) = create_render_compute_bind_groups(
            device,
            &texture,
            &render_bind_group_layout,
            &compute_bind_group_layout,
        );
        let compute_shader = device.create_shader_module(wgpu::include_wgsl!("compute.wgsl"));

        /*
         * we will have one bind group for the texture we are drawing to
         * and one bind group for the model
         */
        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Compute Pipeline Layout"),
                bind_group_layouts: &[&compute_bind_group_layout, &model_bind_group_layout],
                push_constant_ranges: &[],
            });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute Pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader,
            entry_point: "main",
        });

        Pt {
            texture,
            render_bind_group_layout,
            compute_bind_group_layout,
            render_bind_group,
            compute_bind_group,
            compute_pipeline,
            samples_per_pixel,
            size,
            model_bind_group,
        }
    }

    pub fn encode_compute(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Compute Pass"),
        });
        compute_pass.set_pipeline(&self.compute_pipeline);
        compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
        compute_pass.set_bind_group(1, &self.model_bind_group, &[]);

        compute_pass.dispatch_workgroups(self.size.width, self.size.height, self.samples_per_pixel);
    }

    pub fn texture_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.render_bind_group_layout
    }

    pub fn texture_bind_group(&self) -> &wgpu::BindGroup {
        &self.render_bind_group
    }

    pub fn resize(&mut self, device: &wgpu::Device, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;
        self.texture = create_texture(device, new_size, self.samples_per_pixel);
        (self.render_bind_group, self.compute_bind_group) = create_render_compute_bind_groups(
            device,
            &self.texture,
            &self.render_bind_group_layout,
            &self.compute_bind_group_layout,
        );
    }
}

fn create_texture(
    device: &wgpu::Device,
    size: winit::dpi::PhysicalSize<u32>,
    samples_per_pixel: u32,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        size: wgpu::Extent3d {
            width: size.width,
            height: size.height,
            depth_or_array_layers: samples_per_pixel,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D3,
        format: wgpu::TextureFormat::Rgba32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING,
        label: Some("compute_texture"),
        view_formats: &[],
    })
}

fn create_render_compute_bind_groups(
    device: &wgpu::Device,
    texture: &wgpu::Texture,
    render_bg_layout: &wgpu::BindGroupLayout,
    compute_bg_layout: &wgpu::BindGroupLayout,
) -> (wgpu::BindGroup, wgpu::BindGroup) {
    let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("pt_render_bind_group"),
        layout: render_bg_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::TextureView(&texture_view),
        }],
    });

    let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("pt_compute_bind_group"),
        layout: compute_bg_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::TextureView(&texture_view),
        }],
    });
    (render_bind_group, compute_bind_group)
}
