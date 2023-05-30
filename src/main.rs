mod model;
use std::time::{Duration, Instant};
use winit::{
    event::*,
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

struct State {
    window: Window,
    size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    render_pipeline: wgpu::RenderPipeline,
    compute_pipeline: wgpu::ComputePipeline,
    texture_bind_group: wgpu::BindGroup,
    model_bind_group: wgpu::BindGroup,
}

impl State {
    async fn new(window: Window) -> Self {
        let size = window.inner_size();

        // Instance of wgpu.
        // Its primary use is to create Adapters and Surfaces.
        // Does not have to be kept alive.
        let instance = wgpu::Instance::new(Default::default());

        // Draw to this surface, based on a raw window handle
        let surface = unsafe { instance.create_surface(&window) }.unwrap();

        // Handle to a physical graphics and/or compute device.
        // Used for request_device(), then not needed.
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        // Actual connection to the GPU
        let (device, queue) = adapter
            .request_device(&Default::default(), None)
            .await
            .unwrap();

        // This is needed for color format, size, alpha, other stuff
        let surface_config = surface
            .get_default_config(&adapter, size.width, size.height)
            .unwrap();

        surface.configure(&device, &surface_config);
        // We now have a surface we can draw to using our device and queue

        /*
         * We need to create a special texture buffer to draw our result to
         * since we cannot draw directly to the screen from a compute shader
         */
        let compute_texture = device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING,
            label: Some("compute_texture"),
            view_formats: &[],
        });

        let compute_texture_view =
            compute_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let compute_sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("texture_bind_group_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });
        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("texture_bind_group"),
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&compute_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&compute_sampler),
                },
            ],
        });

        let model_bind_group_layout = model::bind_group_layout(&device);
        let model_bind_group =
            model::load(&device, &model_bind_group_layout, "res/cornell_box.obj").unwrap();

        let pt_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("pt_bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba8UnormSrgb,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                }],
            });

        let pt_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("pt_bind_group"),
            layout: &pt_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&compute_texture_view),
            }],
        });

        let compute_shader = device.create_shader_module(wgpu::include_wgsl!("compute.wgsl"));
        let draw_shader = device.create_shader_module(wgpu::include_wgsl!("draw.wgsl"));

        /*
         * we will have one bind group for the texture we are drawing to
         * and one bind group for the model
         */
        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Compute Pipeline Layout"),
                bind_group_layouts: &[&pt_bind_group_layout, &model_bind_group_layout],
                push_constant_ranges: &[],
            });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute Pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader,
            entry_point: "main",
        });

        /*
         * the render pipeline only needs the finished texture
         * and will copy the result onto the screen
         */
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&texture_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &draw_shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &draw_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        State {
            window,
            size,
            surface,
            device,
            queue,
            surface_config,
            render_pipeline,
            compute_pipeline,
            texture_bind_group,
            model_bind_group,
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.surface_config.width = new_size.width;
            self.surface_config.height = new_size.height;
            self.surface.configure(&self.device, &self.surface_config);
        }
    }

    fn update(&mut self, dt: Duration) {
        //todo
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Command Encoder"),
            });
        // first the compute pass will calculate the path tracing result
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute Pass"),
            });
            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.texture_bind_group, &[]);
            compute_pass.set_bind_group(1, &self.model_bind_group, &[]);

            // 4 light paths per pixel
            compute_pass.dispatch_workgroups(self.size.width, self.size.height, 4);
        }

        // then the render pass will copy the result onto the screen
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.texture_bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

async fn run() {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    let mut state = State::new(window).await;
    let mut last_render_time = Instant::now();

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent { ref event, .. } => match event {
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        state: ElementState::Pressed,
                        virtual_keycode: Some(VirtualKeyCode::Escape),
                        ..
                    },
                ..
            } => control_flow.set_exit(),
            WindowEvent::Resized(physical_size) => {
                state.resize(*physical_size);
            }
            WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                state.resize(**new_inner_size);
            }
            _ => (),
        },
        Event::MainEventsCleared => {
            state.window.request_redraw();
        }
        Event::RedrawRequested(_) => {
            let now = Instant::now();
            let dt = now - last_render_time;
            last_render_time = now;
            state.update(dt);
            match state.render() {
                Ok(_) => {}
                // Reconfigure the surface if it's lost or outdated
                Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                    state.resize(state.size)
                }
                // The system is out of memory, we should probably quit
                Err(wgpu::SurfaceError::OutOfMemory) => control_flow.set_exit(),
                // We're ignoring timeouts
                Err(wgpu::SurfaceError::Timeout) => log::warn!("Surface timeout"),
            }
        }
        _ => (),
    });
}

fn main() {
    pollster::block_on(run());
}
