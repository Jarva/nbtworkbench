use std::num::NonZeroU32;

use wgpu::*;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use winit::dpi::PhysicalSize;
use winit::event::*;
use winit::event_loop::{ControlFlow, EventLoop};
use winit::platform::windows::WindowBuilderExtWindows;
use winit::window::{Icon, Window, WindowBuilder};

use crate::{assets, NbtWorkbench};
use crate::vertex_buffer_builder::VertexBufferBuilder;

pub async fn run() {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
                         .with_title("NBT Workbench")
                         .with_transparent(false)
                         .with_inner_size(PhysicalSize::new(620, 420))
                         .with_window_icon(Some(Icon::from_rgba(assets::icon(), assets::ICON_WIDTH, assets::ICON_HEIGHT).expect("valid format")))
                         .with_drag_and_drop(true)
                         .build(&event_loop)
                         .unwrap();
    let mut state = State::new(&window).await;
    let mut workbench = NbtWorkbench::new();

    event_loop.run(move |event, _, control_flow| match event {
        Event::RedrawRequested(window_id) if window_id == window.id() => {
            state.update();
            match state.render(&mut workbench) {
                Ok(_) => {}
                Err(SurfaceError::Lost) => state.surface.configure(&state.device, &state.config),
                Err(SurfaceError::OutOfMemory) => *control_flow = ControlFlow::ExitWithCode(1),
                Err(e) => eprintln!("{:?}", e)
            }
        },
        Event::WindowEvent {
            ref event,
            window_id,
        }

        if window_id == window.id() && !state.input(event, &mut workbench) => match event {
            WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
            WindowEvent::Resized(new_size) => state.resize(&mut workbench, *new_size),
            WindowEvent::ScaleFactorChanged { new_inner_size: new_size, .. } => state.resize(&mut workbench, **new_size),
            _ => {}
        },
        _ => {}
    })
}

struct State {
    surface: Surface,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    render_pipeline: RenderPipeline,
    size: PhysicalSize<u32>,
    diffuse_bind_group: BindGroup
}

impl State {
    async fn new(window: &Window) -> Self {
        let size = window.inner_size();
        let instance = Instance::new(Backends::all());
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance.request_adapter(
            &RequestAdapterOptions {
                power_preference: PowerPreference::LowPower,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false
            }
        ).await.unwrap();
        let (device, queue) = adapter.request_device(&DeviceDescriptor {
                features: Features::empty(),
                limits: if cfg!(target_arch = "wasm32") {
                    Limits::downlevel_webgl2_defaults()
                } else {
                    Limits::default()
                },
                label: None
            },
            None
        ).await.unwrap();
        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_supported_formats(&adapter)[0],
            width: size.width,
            height: size.height,
            present_mode: PresentMode::Immediate
        };
        surface.configure(&device, &config);
        let texture_size = Extent3d {
            width: assets::ATLAS_WIDTH,
            height: assets::ATLAS_HEIGHT,
            depth_or_array_layers: 1,
        };
        let diffuse_texture = device.create_texture(
            &TextureDescriptor {
                size: texture_size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba8UnormSrgb,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                label: Some("Diffuse Texture"),
            }
        );
        queue.write_texture(
            ImageCopyTexture {
                texture: &diffuse_texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All
            },
            assets::ATLAS,
            ImageDataLayout {
                offset: 0,
                bytes_per_row: NonZeroU32::new(4 * assets::ATLAS_WIDTH), // must be divisible by 256, 1024 % 256 == 0
                rows_per_image: NonZeroU32::new(assets::ATLAS_HEIGHT)
            },
            texture_size
        );
        let diffuse_texture_view = diffuse_texture.create_view(&TextureViewDescriptor::default());
        let diffuse_sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("Diffuse Sampler"),
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            address_mode_w: AddressMode::Repeat,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });
        let texture_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        view_dimension: TextureViewDimension::D2,
                        sample_type: TextureSampleType::Float { filterable: true }
                    },
                    count: None
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None
                }
            ],
            label: Some("Texture Bind Group Layout")
        });
        let diffuse_bind_group = device.create_bind_group(&BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&diffuse_texture_view)
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&diffuse_sampler)
                }
            ],
            label: Some("Diffuse Bind Group")
        });
        let shader = device.create_shader_module(include_wgsl!("assets/shader.wgsl"));
        let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&texture_bind_group_layout],
            push_constant_ranges: &[],
        });
        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[
                    VertexBufferLayout {
                        array_stride: 24 as BufferAddress,
                        step_mode: VertexStepMode::Vertex,
                        attributes: &vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Uint32]
                    }
                ]
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState {
                    format: config.format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL
                })]
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                polygon_mode: PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false
            },
            multiview: None
        });

        Self {
            surface,
            device,
            queue,
            config,
            render_pipeline,
            size,
            diffuse_bind_group
        }
    }

    fn resize(&mut self, workbench: &mut NbtWorkbench, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            workbench.window_height(new_size.height);
        }
    }

    fn input(&mut self, event: &WindowEvent, workbench: &mut NbtWorkbench) -> bool {
        match event {
            WindowEvent::Resized(_) => false,
            WindowEvent::Moved(_) => false,
            WindowEvent::CloseRequested => false,
            WindowEvent::Destroyed => false,
            WindowEvent::DroppedFile(file) => workbench.on_open_file(file),
            WindowEvent::HoveredFile(_) => false,
            WindowEvent::HoveredFileCancelled => false,
            WindowEvent::ReceivedCharacter(_) => false,
            WindowEvent::Focused(_) => false,
            WindowEvent::KeyboardInput { input, .. } => workbench.on_key_input(input),
            WindowEvent::ModifiersChanged(_) => false,
            WindowEvent::CursorMoved { position, .. } => workbench.on_cursor_move(position),
            WindowEvent::CursorEntered { .. } => false,
            WindowEvent::CursorLeft { .. } => false,
            WindowEvent::MouseWheel { delta, .. } => workbench.on_scroll(delta),
            WindowEvent::MouseInput { state, button, .. } => workbench.on_mouse_input(state, button),
            WindowEvent::TouchpadPressure { .. } => false,
            WindowEvent::AxisMotion { .. } => false,
            WindowEvent::Touch(_) => false,
            WindowEvent::ScaleFactorChanged { .. } => false,
            WindowEvent::ThemeChanged(_) => false,
            WindowEvent::Ime(_) => false,
            WindowEvent::Occluded(_) => false
        }
    }

    fn update(&mut self) {

    }

    fn render(&mut self, workbench: &mut NbtWorkbench) -> Result<(), SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Render Encoder")
        });

        {
            let vertex_buffer;
            let index_buffer;
            {
                let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                    label: Some("Render Pass"),
                    color_attachments: &[Some(RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(Color {
                                r: 0.013,
                                g: 0.013,
                                b: 0.013,
                                a: 1.0
                            })/*Load*/,
                            store: true
                        }
                    })],
                    depth_stencil_attachment: None
                });

                render_pass.set_pipeline(&self.render_pipeline);
                render_pass.set_bind_group(0, &self.diffuse_bind_group, &[]);

                let mut builder = VertexBufferBuilder::new(&self.size, assets::ATLAS_WIDTH, assets::ATLAS_HEIGHT, workbench.scroll());
                workbench.render(&mut builder);

                vertex_buffer = self.device.create_buffer_init(&BufferInitDescriptor {
                    label: Some("Vertex Buffer"),
                    contents: builder.vertices(),
                    usage: BufferUsages::VERTEX
                });

                index_buffer = self.device.create_buffer_init(&BufferInitDescriptor {
                    label: Some("Index Buffer"),
                    contents: builder.indices(),
                    usage: BufferUsages::INDEX
                });

                render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                render_pass.set_index_buffer(index_buffer.slice(..), IndexFormat::Uint16);

                render_pass.draw_indexed(0..builder.indices_len(), 0, 0..1);
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}