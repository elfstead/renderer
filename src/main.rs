mod model;
mod pt;
use pollster::FutureExt;
use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::*,
    window::{Window, WindowId},
};

#[derive(Default)]
struct App {
    state: Option<State>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = event_loop
            .create_window(Window::default_attributes().with_title("Renderer"))
            .unwrap();
        self.state = Some(State::new(window).block_on());
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        logical_key: Key::Named(NamedKey::Escape),
                        ..
                    },
                ..
            } => event_loop.exit(),
            WindowEvent::Resized(physical_size) => {
                self.state.as_mut().unwrap().resize(physical_size);
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = now - self.state.as_ref().unwrap().last_render_time;
                self.state.as_mut().unwrap().last_render_time = now;
                println!("frametime: {} ms", dt.as_millis());
                self.state.as_mut().unwrap().update(dt);
                match self.state.as_mut().unwrap().render() {
                    Ok(_) => {}
                    // Reconfigure the surface if it's lost or outdated
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        let size = self.state.as_ref().unwrap().size;
                        self.state.as_mut().unwrap().resize(size);
                    }
                    // The system is out of memory, we should probably quit
                    Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                    // We're ignoring timeouts
                    Err(wgpu::SurfaceError::Timeout) => log::warn!("Surface timeout"),
                }
            }
            _ => (),
        }
    }
        fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // tbh should remove this and decouple background math from refresh rate
        match self.state.as_ref() { Some(state) => {
            state.window.request_redraw();
        } _ => {}}
    }
}
async fn run() {
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();
    let mut app = App::default();

    _ = event_loop.run_app(&mut app);
}

struct State {
    window: Arc<Window>,
    size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    render_pipeline: wgpu::RenderPipeline,
    pt: pt::Pt,
    last_render_time: Instant,
}

impl State {
    async fn new(window: Window) -> Self {
        let window_arc = Arc::new(window);
        let size = window_arc.inner_size();

        // Instance of wgpu.
        // Its primary use is to create Adapters and Surfaces.
        // Does not have to be kept alive.
        let instance = wgpu::Instance::new(Default::default());

        // Draw to this surface, based on a raw window handle
        let surface = instance.create_surface(window_arc.clone()).unwrap();

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

        let pt = pt::Pt::new(&device, size);

        let draw_shader = device.create_shader_module(wgpu::include_wgsl!("draw.wgsl"));

        /*
         * the render pipeline only needs the finished texture
         * and will copy the result onto the screen
         */
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[pt.bind_group_layout()],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &draw_shader,
                entry_point: "vs_main",
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &draw_shader,
                entry_point: "fs_main",
                compilation_options: Default::default(),
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
            cache: None,
        });

        State {
            window: window_arc,
            size,
            surface,
            device,
            queue,
            surface_config,
            render_pipeline,
            pt,
            last_render_time: Instant::now(),
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.surface_config.width = new_size.width;
            self.surface_config.height = new_size.height;
            self.surface.configure(&self.device, &self.surface_config);
            self.pt.resize(&self.device, new_size);
        }
    }

    fn update(&mut self, _dt: Duration) {
        //todo: add movement
    }

    // can make this non-mutating if I build the pt continuously in a separate thread
    // (or rebuild from scratch every frame, in the case of making a real time version)
    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        self.pt.next_frame(&self.queue);

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Command Encoder"),
            });
        // first the compute pass will calculate the path tracing result
        self.pt.encode_compute(&mut encoder);

        // then the render pass will copy the result onto the screen
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, self.pt.bind_group(), &[]);
            render_pass.draw(0..3, 0..1); // one triangle that covers whole screen
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

fn main() {
    pollster::block_on(run());
}
