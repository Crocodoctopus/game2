use crate::client::{GameRenderDesc, SpriteRenderDesc};
use crate::window::{InputEvent, Window};
use futures::executor::block_on;
use nalgebra_glm::*;
use std::path::Path;
use wgpu::util::DeviceExt;
use wgpu::*;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Mat4([[f32; 4]; 4]);

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vec4([f32; 4]);

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct TileVertexInput {
    tile_xyz: [f32; 3],
    tile_uv: [f32; 2],
    mask_uv: [f32; 2],
}

impl TileVertexInput {
    const ATTRIB: [VertexAttribute; 3] = vertex_attr_array![
        0 => Float32x3,
        1 => Float32x2,
        2 => Float32x2
    ];

    fn buffer_layout<'a>() -> VertexBufferLayout<'a> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as _,
            step_mode: VertexStepMode::Vertex,
            attributes: &Self::ATTRIB,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct LightVertexInput {
    light_xy: [f32; 2],
    light_uv: [f32; 2],
}

impl LightVertexInput {
    const ATTRIB: [VertexAttribute; 2] = vertex_attr_array![
        0 => Float32x2,
        1 => Float32x2,
    ];

    fn buffer_layout<'a>() -> VertexBufferLayout<'a> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as _,
            step_mode: VertexStepMode::Vertex,
            attributes: &Self::ATTRIB,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct SpriteVertexInput {
    sprite_xy: [f32; 2],
    sprite_uv: [f32; 2],
}

#[allow(dead_code)]
pub struct GameRenderState<'a> {
    // State.
    surface_config: SurfaceConfiguration,
    surface: Surface<'a>,
    device: Device,
    queue: Queue,

    // Textures.
    sprite_tex: (Texture, TextureView),
    tile_sprite_tex: (Texture, TextureView),
    tile_mask_tex: (Texture, TextureView),
    light_tex: (Texture, TextureView),

    // General purpose IBO.
    quad_ibo: Buffer,

    // Misc bind group.
    misc_bind_group: BindGroup,
    view_uniform: Buffer,

    // Tile rendering.
    tile_pipeline: RenderPipeline,
    fg_const_uniform: Buffer,
    bg_const_uniform: Buffer,
    fg_bind_group: BindGroup,
    bg_bind_group: BindGroup,

    // Light rendering.
    light_pipeline: RenderPipeline,
    light_bind_group: BindGroup,

    // Sprite rendering.
    sprite_pipeline: RenderPipeline,
    sprite_bind_group: BindGroup,
}

impl<'a> GameRenderState<'a> {
    pub fn new(_root: &'static Path, window: &'a Window) -> Self {
        // General initialization of render state.
        let (surface, device, queue, surface_config) = {
            // Instance.
            let instance = Instance::new(InstanceDescriptor {
                backends: Backends::all(),
                ..Default::default()
            });

            // Surface.
            let surface = instance.create_surface(&window.window).unwrap();

            // Physical device.
            let physical_device = block_on(instance.request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::LowPower,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            }))
            .expect("Could not find a suitable GPU.");

            // Logical device and command queue.
            let (device, queue) = block_on(physical_device.request_device(
                &DeviceDescriptor {
                    required_features: Features::empty(),
                    ..Default::default()
                },
                None,
            ))
            .unwrap();

            //
            let surface_config = SurfaceConfiguration {
                usage: TextureUsages::RENDER_ATTACHMENT,
                format: TextureFormat::Bgra8Unorm,
                width: 1280,
                height: 720,
                present_mode: PresentMode::Fifo,
                desired_maximum_frame_latency: 1,
                alpha_mode: CompositeAlphaMode::Auto,
                view_formats: vec![],
            };
            surface.configure(&device, &surface_config);

            (surface, device, queue, surface_config)
        };

        #[rustfmt::skip]
        use image::GenericImageView;
        let create_wgpu_texture = |(width, height), format, data: &[u8]| {
            device.create_texture_with_data(
                &queue,
                &TextureDescriptor {
                    label: None,
                    size: Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    format,
                    usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                    view_formats: &[],
                },
                util::TextureDataOrder::LayerMajor,
                data,
            )
        };

        let sprite_tex = {
            let texture =
                image::load_from_memory(include_bytes!("../../resources/tile_sheet.png")).unwrap();
            let size = texture.dimensions();
            let pixels = texture.into_rgba8();
            let texture = create_wgpu_texture(size, TextureFormat::Rgba8Unorm, &pixels);
            let view = texture.create_view(&<_>::default());
            (texture, view)
        };

        let tile_sprite_tex = {
            let texture =
                image::load_from_memory(include_bytes!("../../resources/tile_sheet.png")).unwrap();
            let size = texture.dimensions();
            let pixels = texture.into_rgba8();
            let texture = create_wgpu_texture(size, TextureFormat::Rgba8Unorm, &pixels);
            let view = texture.create_view(&<_>::default());
            (texture, view)
        };

        let tile_mask_tex = {
            let texture =
                image::load_from_memory(include_bytes!("../../resources/mask_sheet.png")).unwrap();
            let size = texture.dimensions();
            let pixels = texture.into_luma8();
            let texture = create_wgpu_texture(size, TextureFormat::R8Uint, &pixels);
            let view = texture.create_view(&<_>::default());
            (texture, view)
        };

        let light_tex = {
            let texture = device.create_texture(&TextureDescriptor {
                label: Some("Light Texture"),
                size: Extent3d {
                    width: 512,
                    height: 512,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba8Uint,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                view_formats: &[],
            });
            let view = texture.create_view(&<_>::default());
            (texture, view)
        };

        // Generic index buffer.
        let quad_ibo = {
            #[rustfmt::skip]
            let ibo_data: Vec<u16> = (0..13107)
                .into_iter()
                .flat_map(|i| [i * 4 + 0, i * 4 + 3, i * 4 + 1, i * 4 + 2, u16::MAX])
                .collect();
            assert_eq!(ibo_data.len(), 65535);

            let buffer = device.create_buffer_init(&util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(&ibo_data),
                usage: BufferUsages::INDEX,
            });

            buffer
        };

        // Create camera buffer.
        let view_uniform = device.create_buffer(&BufferDescriptor {
            label: Some("View Uniform"),
            size: std::mem::size_of::<Mat4>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Misc bind group.
        let misc_bind_group = {
            let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Misc Bind Group Layout"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

            let group = device.create_bind_group(&BindGroupDescriptor {
                label: Some("Misc Bind Group"),
                layout: &layout,
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: view_uniform.as_entire_binding(),
                }],
            });

            (group, layout)
        };

        // Const uniforms.
        let fg_const_uniform = device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("FG Const Uniform"),
            usage: BufferUsages::UNIFORM,
            contents: bytemuck::cast_slice(&[Vec4([1.0, 1.0, 1.0, 1.0])]),
        });
        let bg_const_uniform = device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("FG Const Uniform"),
            usage: BufferUsages::UNIFORM,
            contents: bytemuck::cast_slice(&[Vec4([0.6, 0.6, 0.7, 1.0])]),
        });

        // Create tile rendering pipeline.
        let (tile_pipeline, fg_bind_group, bg_bind_group) = {
            // Shader.
            let shader = device.create_shader_module(include_wgsl!("shaders/tile.wgsl"));

            // Bind group.
            let (fg_bind_group, bg_bind_group, bind_group_layout) = {
                let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                    entries: &[
                        BindGroupLayoutEntry {
                            binding: 0,
                            visibility: ShaderStages::FRAGMENT,
                            ty: BindingType::Texture {
                                multisampled: false,
                                view_dimension: TextureViewDimension::D2,
                                sample_type: TextureSampleType::Float { filterable: false },
                            },
                            count: None,
                        },
                        BindGroupLayoutEntry {
                            binding: 1,
                            visibility: ShaderStages::FRAGMENT,
                            ty: BindingType::Texture {
                                multisampled: false,
                                view_dimension: TextureViewDimension::D2,
                                sample_type: TextureSampleType::Uint,
                            },
                            count: None,
                        },
                        BindGroupLayoutEntry {
                            binding: 2,
                            visibility: ShaderStages::FRAGMENT,
                            ty: BindingType::Buffer {
                                ty: BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                    label: Some("Tile Bind Group Layout"),
                });

                let fg_group = device.create_bind_group(&BindGroupDescriptor {
                    layout: &layout,
                    entries: &[
                        BindGroupEntry {
                            binding: 0,
                            resource: BindingResource::TextureView(&tile_sprite_tex.1),
                        },
                        BindGroupEntry {
                            binding: 1,
                            resource: BindingResource::TextureView(&tile_mask_tex.1),
                        },
                        BindGroupEntry {
                            binding: 2,
                            resource: fg_const_uniform.as_entire_binding(),
                        },
                    ],
                    label: Some("Tile FG Bind Group"),
                });

                let bg_group = device.create_bind_group(&BindGroupDescriptor {
                    layout: &layout,
                    entries: &[
                        BindGroupEntry {
                            binding: 0,
                            resource: BindingResource::TextureView(&tile_sprite_tex.1),
                        },
                        BindGroupEntry {
                            binding: 1,
                            resource: BindingResource::TextureView(&tile_mask_tex.1),
                        },
                        BindGroupEntry {
                            binding: 2,
                            resource: bg_const_uniform.as_entire_binding(),
                        },
                    ],
                    label: Some("Tile BG Bind Group"),
                });

                (fg_group, bg_group, layout)
            };

            // Pipeline layout.
            let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&misc_bind_group.1, &bind_group_layout],
                push_constant_ranges: &[],
            });

            // Render pipeline.
            let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Tile Render Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &[TileVertexInput::buffer_layout()],
                },
                fragment: Some(FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    targets: &[Some(ColorTargetState {
                        format: surface_config.format,
                        blend: Some(BlendState::REPLACE),
                        write_mask: ColorWrites::ALL,
                    })],
                }),
                primitive: PrimitiveState {
                    topology: PrimitiveTopology::TriangleStrip,
                    strip_index_format: Some(IndexFormat::Uint16),
                    front_face: FrontFace::Ccw,
                    cull_mode: None,
                    polygon_mode: PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
            });

            (pipeline, fg_bind_group, bg_bind_group)
        };

        let (light_pipeline, light_bind_group) = {
            // Shader.
            let shader = device.create_shader_module(include_wgsl!("shaders/light.wgsl"));

            // Bind group.
            let bind_group = {
                let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: Some("Light Bind Group Layout"),
                    entries: &[BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Uint,
                        },
                        count: None,
                    }],
                });

                let group = device.create_bind_group(&BindGroupDescriptor {
                    label: Some("Light Bind Group"),
                    layout: &layout,
                    entries: &[BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(&light_tex.1),
                    }],
                });

                (group, layout)
            };

            // Pipeline layout.
            let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Light Pipeline Layout"),
                bind_group_layouts: &[&misc_bind_group.1, &bind_group.1],
                push_constant_ranges: &[],
            });

            // Render pipeline.
            let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Light Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &[LightVertexInput::buffer_layout()],
                },
                fragment: Some(FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    targets: &[Some(ColorTargetState {
                        format: surface_config.format,
                        blend: Some(BlendState {
                            color: BlendComponent {
                                src_factor: BlendFactor::Dst,
                                dst_factor: BlendFactor::Zero,
                                operation: BlendOperation::Add,
                            },
                            alpha: BlendComponent::default(),
                        }),
                        write_mask: ColorWrites::ALL,
                    })],
                }),
                primitive: PrimitiveState {
                    topology: PrimitiveTopology::TriangleStrip,
                    strip_index_format: Some(IndexFormat::Uint16),
                    front_face: FrontFace::Ccw,
                    cull_mode: None,
                    polygon_mode: PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
            });

            (pipeline, bind_group)
        };

        let (sprite_pipeline, sprite_bind_group) = {
            // Shader.
            let shader = device.create_shader_module(include_wgsl!("shaders/sprite.wgsl"));

            // Bind group.
            let bind_group = {
                let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: Some("Sprite Bind Group Layout"),
                    entries: &[BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    }],
                });

                let group = device.create_bind_group(&BindGroupDescriptor {
                    label: Some("Sprite Bind Group"),
                    layout: &layout,
                    entries: &[BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(&sprite_tex.1),
                    }],
                });

                (group, layout)
            };

            // Pipeline layout.
            let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Sprite Pipeline Layout"),
                bind_group_layouts: &[&misc_bind_group.1, &bind_group.1],
                push_constant_ranges: &[],
            });

            // Render pipeline.
            let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Sprite Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &[LightVertexInput::buffer_layout()],
                },
                fragment: Some(FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    targets: &[Some(ColorTargetState {
                        format: surface_config.format,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    })],
                }),
                primitive: PrimitiveState {
                    topology: PrimitiveTopology::TriangleStrip,
                    strip_index_format: Some(IndexFormat::Uint16),
                    front_face: FrontFace::Ccw,
                    cull_mode: None,
                    polygon_mode: PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
            });

            (pipeline, bind_group)
        };

        Self {
            surface,
            device,
            queue,
            surface_config,

            quad_ibo,

            misc_bind_group: misc_bind_group.0,
            view_uniform,

            sprite_tex,
            tile_sprite_tex,
            tile_mask_tex,
            light_tex,

            tile_pipeline,
            fg_const_uniform,
            bg_const_uniform,
            fg_bind_group,
            bg_bind_group,

            light_pipeline,
            light_bind_group: light_bind_group.0,

            sprite_pipeline,
            sprite_bind_group: sprite_bind_group.0,
        }
    }

    pub fn handle_events<'e>(
        &mut self,
        input_events: impl Iterator<Item = &'e InputEvent>,
    ) -> bool {
        for &event in input_events {
            match event {
                InputEvent::WindowClose => return true,

                InputEvent::WindowResize { width, height } => {
                    self.surface_config.width = width as u32;
                    self.surface_config.height = height as u32;
                    self.surface.configure(&self.device, &self.surface_config);
                }

                // Most events are ignored.
                _ => {}
            }
        }

        false
    }

    pub fn render(&mut self, _ts: u64, game_render_desc: &GameRenderDesc) {
        // Whisked away to a far off place.
        self.process_view_matrix(&game_render_desc);
        let light_vertex_input = self.process_light_state(&game_render_desc);
        let (fg_vertex_input, fg_count, bg_vertex_input, bg_count) =
            self.process_tile_state(&game_render_desc);
        let (sprite_vertex_input, sprite_count) = self.process_sprite_state(&game_render_desc);

        // Begin rendering.
        let output = self.surface.get_current_texture().unwrap();
        let view = output.texture.create_view(&<_>::default());
        let mut encoder = self.device.create_command_encoder(&<_>::default());
        let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color {
                        r: 0x15 as f64 / 255.,
                        g: 0x9F as f64 / 255.,
                        b: 0xEA as f64 / 255.,
                        a: 1.0,
                    }),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        // Generic IBO and misc group.
        render_pass.set_index_buffer(self.quad_ibo.slice(..), IndexFormat::Uint16);
        render_pass.set_bind_group(0, &self.misc_bind_group, &[]);

        // Tile rendering.
        {
            // Pipeline and tile bind group are shared.
            render_pass.set_pipeline(&self.tile_pipeline);

            // BG Tile Rendering.
            render_pass.set_bind_group(1, &self.bg_bind_group, &[]);
            render_pass.set_vertex_buffer(0, bg_vertex_input.slice(..));
            render_pass.draw_indexed(0..bg_count * 5, 0, 0..1);

            // FG Tile Rendering.
            render_pass.set_bind_group(1, &self.fg_bind_group, &[]);
            render_pass.set_vertex_buffer(0, fg_vertex_input.slice(..));
            render_pass.draw_indexed(0..fg_count * 5, 0, 0..1);
        }

        // Light rendering.
        {
            render_pass.set_pipeline(&self.light_pipeline);
            render_pass.set_bind_group(1, &self.light_bind_group, &[]);
            render_pass.set_vertex_buffer(0, light_vertex_input.slice(..));
            render_pass.draw_indexed(0..4, 0, 0..1);
        }
        
        // Sprite rendering.
        {
            render_pass.set_pipeline(&self.sprite_pipeline);
            render_pass.set_bind_group(1, &self.sprite_bind_group, &[]);
            render_pass.set_vertex_buffer(0, sprite_vertex_input.slice(..));
            render_pass.draw_indexed(0..sprite_count * 5, 0, 0..1);
        }

        // End rendering.
        drop(render_pass);
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }

    fn process_view_matrix(&mut self, game_render_desc: &GameRenderDesc) {
        // Calculate view matrix.
        let view = {
            let view = Mat3::identity();
            let view = view
                * scaling2d(&Vec2::new(
                    2. / game_render_desc.viewport_w,
                    -2. / game_render_desc.viewport_h,
                ));
            let view = view
                * translation2d(&Vec2::new(
                    -game_render_desc.viewport_x - game_render_desc.viewport_w / 2.,
                    -game_render_desc.viewport_y - game_render_desc.viewport_h / 2.,
                ));
            view
        };

        self.queue.write_buffer(
            &self.view_uniform,
            0,
            bytemuck::cast_slice(&[Mat4(nalgebra_glm::mat3_to_mat4(&view).into())]),
        );
    }

    fn process_light_state(&mut self, game_render_desc: &GameRenderDesc) -> Buffer {
        // Calculate light data.
        let rgba: Vec<u8> = (0..game_render_desc.light_w * game_render_desc.light_h)
            .into_iter()
            .flat_map(|i| {
                [
                    game_render_desc.r_channel[i],
                    game_render_desc.g_channel[i],
                    game_render_desc.b_channel[i],
                    255,
                ]
            })
            .collect();
        let light_x = game_render_desc.light_x as f32;
        let light_y = game_render_desc.light_y as f32;
        let light_w = game_render_desc.light_w as f32;
        let light_h = game_render_desc.light_h as f32;
        let light_vertices = [
            LightVertexInput {
                light_xy: [light_x * 16., light_y * 16.],
                light_uv: [0., 0.],
            },
            LightVertexInput {
                light_xy: [(light_x + light_w) * 16., light_y * 16.],
                light_uv: [light_w, 0.],
            },
            LightVertexInput {
                light_xy: [(light_x + light_w) * 16., (light_y + light_h) * 16.],
                light_uv: [light_w, light_h],
            },
            LightVertexInput {
                light_xy: [light_x * 16., (light_y + light_h) * 16.],
                light_uv: [0., light_h],
            },
        ];

        // Upload light texture.
        self.queue.write_texture(
            ImageCopyTextureBase {
                texture: &self.light_tex.0,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            &rgba,
            ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * game_render_desc.light_w as u32),
                rows_per_image: Some(game_render_desc.light_h as u32),
            },
            Extent3d {
                width: game_render_desc.light_w as u32,
                height: game_render_desc.light_h as u32,
                depth_or_array_layers: 1,
            },
        );

        // Upload light vbo data.
        let light_vertex_input = self.device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("Light Vertex Input Buffer"),
            contents: bytemuck::cast_slice(&light_vertices),
            usage: BufferUsages::VERTEX,
        });

        light_vertex_input
    }

    fn process_sprite_state(&mut self, game_render_desc: &GameRenderDesc) -> (Buffer, u32) {
        let mut sprites = Vec::with_capacity(4 * game_render_desc.sprites.len());

        for &SpriteRenderDesc { x, y, u, v, w, h } in game_render_desc.sprites.iter() {
            sprites.extend_from_slice(&[
                SpriteVertexInput {
                    sprite_xy: [x, y],
                    sprite_uv: [u, v],
                },
                SpriteVertexInput {
                    sprite_xy: [x + w, y],
                    sprite_uv: [u + w, v],
                },
                SpriteVertexInput {
                    sprite_xy: [x + w, y + h],
                    sprite_uv: [u + w, v + h],
                },
                SpriteVertexInput {
                    sprite_xy: [x, y + h],
                    sprite_uv: [u, v + h],
                },
            ]);
        }

        // Upload tile vbo data.
        let sprite_vertex_input = self.device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("Sprite Vertex Buffer"),
            contents: bytemuck::cast_slice(&sprites),
            usage: BufferUsages::VERTEX,
        });

        (sprite_vertex_input, sprites.len() as u32 / 4)
    }

    fn process_tile_state(
        &mut self,
        game_render_desc: &GameRenderDesc,
    ) -> (Buffer, u32, Buffer, u32) {
        // Calculate tile vertex data.
        let max_tiles = (game_render_desc.tiles_w - 2) * (game_render_desc.tiles_h - 2);
        let mut fg_vertex_tiles = Vec::with_capacity(4 * max_tiles);
        let mut bg_vertex_tiles = Vec::with_capacity(4 * max_tiles);
        if max_tiles > 0 {
            // Calculate tile data and upload to GPU.
            let tile_texture_properties_lookup = &crate::shared::TILE_TEXTURE_PROPERTIES;
            let stride = game_render_desc.tiles_w;
            'calc_tiles: {
                for y in 1..game_render_desc.tiles_h - 1 {
                    'x: for x in 1..game_render_desc.tiles_w - 1 {
                        let index = x + y * game_render_desc.tiles_w;

                        // Fill FG.
                        'skip_fg: {
                            let tile_texture_properties = tile_texture_properties_lookup
                                [game_render_desc.fg_tiles[index].0 as usize];

                            // Get texture UV.
                            let u = tile_texture_properties.u;
                            let v = tile_texture_properties.v;

                            // If not visible, skip.
                            if (u, v) == (0., 0.) {
                                break 'skip_fg;
                            }

                            // Get depth.
                            let depth = tile_texture_properties.depth;

                            // Calculate position.
                            let x = 16. * (x + game_render_desc.tiles_x) as f32;
                            let y = 16. * (y + game_render_desc.tiles_y) as f32;
                            let z = depth as f32;

                            // Calculate mask UV.
                            #[rustfmt::skip]
                            let mask_u = [ index - stride, index - stride + 1, index + 1, index + stride + 1 ]
                                .into_iter()
                                .rev()
                                .map(|index| game_render_desc.fg_tiles[index].0)
                                .map(|tile| tile_texture_properties_lookup[tile as usize].depth)
                                .map(|dep| (depth > dep) as u8)
                                .reduce(|acc, bit| (acc << 1) | bit)
                                .map(|out| (out << 2) as f32)
                                .unwrap();
                            #[rustfmt::skip]
                            let mask_v  = [index + stride, index + stride - 1, index - 1, index - stride - 1 ]
                                .into_iter()
                                .rev()
                                .map(|index| game_render_desc.fg_tiles[index].0)
                                .map(|tile| tile_texture_properties_lookup[tile as usize].depth)
                                .map(|dep| (depth > dep) as u8)
                                .reduce(|acc, bit| (acc << 1) | bit)
                                .map(|out| (out << 2) as f32)
                                .unwrap();

                            fg_vertex_tiles.extend_from_slice(&[
                                TileVertexInput {
                                    tile_xyz: [x - 8., y - 8., z],
                                    tile_uv: [u, v],
                                    mask_uv: [mask_u, mask_v],
                                },
                                TileVertexInput {
                                    tile_xyz: [x + 16. + 8., y - 8., z],
                                    tile_uv: [u + 16., v],
                                    mask_uv: [mask_u + 4., mask_v],
                                },
                                TileVertexInput {
                                    tile_xyz: [x + 16. + 8., y + 16. + 8., z],
                                    tile_uv: [u + 16., v + 16.],
                                    mask_uv: [mask_u + 4., mask_v + 4.],
                                },
                                TileVertexInput {
                                    tile_xyz: [x - 8., y + 16. + 8., z],
                                    tile_uv: [u, v + 16.],
                                    mask_uv: [mask_u, mask_v + 4.],
                                },
                            ]);

                            // Skip check bg tile.
                            continue 'x;
                        }

                        // Fill FG.
                        'skip_bg: {
                            let tile_texture_properties = tile_texture_properties_lookup
                                [game_render_desc.bg_tiles[index].0 as usize];

                            // Get texture UV.
                            let u = tile_texture_properties.u;
                            let v = tile_texture_properties.v;

                            // If not visible, skip.
                            if (u, v) == (0., 0.) {
                                break 'skip_bg;
                            }

                            // Get depth.
                            let depth = tile_texture_properties.depth;

                            // Calculate position.
                            let x = 16. * (x + game_render_desc.tiles_x) as f32;
                            let y = 16. * (y + game_render_desc.tiles_y) as f32;
                            let z = depth as f32;

                            // Calculate mask UV.
                            #[rustfmt::skip]
                            let mask_u = [ index - stride, index - stride + 1, index + 1, index + stride + 1 ]
                                .into_iter()
                                .rev()
                                .map(|index| game_render_desc.bg_tiles[index].0)
                                .map(|tile| tile_texture_properties_lookup[tile as usize].depth)
                                .map(|dep| (depth > dep) as u8)
                                .reduce(|acc, bit| (acc << 1) | bit)
                                .map(|out| (out << 2) as f32)
                                .unwrap();
                            #[rustfmt::skip]
                            let mask_v  = [index + stride, index + stride - 1, index - 1, index - stride - 1 ]
                                .into_iter()
                                .rev()
                                .map(|index| game_render_desc.bg_tiles[index].0)
                                .map(|tile| tile_texture_properties_lookup[tile as usize].depth)
                                .map(|dep| (depth > dep) as u8)
                                .reduce(|acc, bit| (acc << 1) | bit)
                                .map(|out| (out << 2) as f32)
                                .unwrap();

                            bg_vertex_tiles.extend_from_slice(&[
                                TileVertexInput {
                                    tile_xyz: [x - 8., y - 8., z],
                                    tile_uv: [u, v],
                                    mask_uv: [mask_u, mask_v],
                                },
                                TileVertexInput {
                                    tile_xyz: [x + 16. + 8., y - 8., z],
                                    tile_uv: [u + 16., v],
                                    mask_uv: [mask_u + 4., mask_v],
                                },
                                TileVertexInput {
                                    tile_xyz: [x + 16. + 8., y + 16. + 8., z],
                                    tile_uv: [u + 16., v + 16.],
                                    mask_uv: [mask_u + 4., mask_v + 4.],
                                },
                                TileVertexInput {
                                    tile_xyz: [x - 8., y + 16. + 8., z],
                                    tile_uv: [u, v + 16.],
                                    mask_uv: [mask_u, mask_v + 4.],
                                },
                            ]);

                            // Skip check bg tile.
                            continue 'x;
                        }
                    }
                }

                break 'calc_tiles;
            };
        }

        // Upload fg tile vbo data.
        let fg_vertex_input = self.device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("FG Vertex Buffer"),
            contents: bytemuck::cast_slice(&fg_vertex_tiles),
            usage: BufferUsages::VERTEX,
        });

        // Upload tile vbo data.
        let bg_vertex_input = self.device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("BG Vertex Buffer"),
            contents: bytemuck::cast_slice(&bg_vertex_tiles),
            usage: BufferUsages::VERTEX,
        });

        (
            fg_vertex_input,
            fg_vertex_tiles.len() as u32 / 4,
            bg_vertex_input,
            bg_vertex_tiles.len() as u32 / 4,
        )
    }
}
