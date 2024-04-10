use crate::client::GameFrame;
use crate::window::Window;
use futures::executor::block_on;
use nalgebra_glm::*;
use std::path::Path;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ViewUniform([[f32; 4]; 4]);

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct TileVertexInput {
    tile_xyz: [f32; 3],
    tile_uv: [f32; 2],
    mask_uv: [f32; 2],
}

impl TileVertexInput {
    const ATTRIB: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![
        0 => Float32x3,
        1 => Float32x2,
        2 => Float32x2
    ];
}

pub struct GameRenderState<'a> {
    // State.
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    format: wgpu::TextureFormat,

    // General purpose IBO.
    quad_ibo: wgpu::Buffer,

    // Misc bind group.
    misc_bind_group: wgpu::BindGroup,
    view_uniform: wgpu::Buffer,
    generic_sampler: wgpu::Sampler,

    // Tile rendering.
    tile_pipeline: wgpu::RenderPipeline,
    tile_bind_group: wgpu::BindGroup,

    tile_sprite_tex: (wgpu::Texture, wgpu::TextureView),
    tile_mask_tex: (wgpu::Texture, wgpu::TextureView),
}

impl<'a> GameRenderState<'a> {
    pub fn new(_root: &'static Path, window: &'a Window) -> Self {
        // General initialization of render state.
        let (surface, device, queue, format) = {
            // Instance.
            let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
                backends: wgpu::Backends::all(),
                ..Default::default()
            });

            // Surface.
            let surface = instance.create_surface(&window.window).unwrap();

            // Physical device.
            let physical_device =
                block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::LowPower,
                    compatible_surface: Some(&surface),
                    force_fallback_adapter: false,
                }))
                .expect("Could not find a suitable GPU.");

            // Logical device and command queue.
            let (device, queue) = block_on(physical_device.request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    ..Default::default()
                },
                None,
            ))
            .unwrap();

            //
            let format = wgpu::TextureFormat::Bgra8UnormSrgb;
            surface.configure(
                &device,
                &wgpu::SurfaceConfiguration {
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    format,
                    width: 1280,
                    height: 720,
                    present_mode: wgpu::PresentMode::AutoVsync,
                    desired_maximum_frame_latency: 3,
                    alpha_mode: wgpu::CompositeAlphaMode::Auto,
                    view_formats: vec![],
                },
            );

            (surface, device, queue, format)
        };

        // Load texture data
        let tile_sprite_tex = {
            use image::GenericImageView;
            let texture =
                image::load_from_memory(include_bytes!("../../resources/tile_sheet.png")).unwrap();
            let (width, height) = texture.dimensions();
            let pixels = texture.into_rgba8();

            let texture = device.create_texture_with_data(
                &queue,
                &wgpu::TextureDescriptor {
                    label: Some("Tile Sheet Texture"),
                    size: wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                },
                wgpu::util::TextureDataOrder::LayerMajor,
                &pixels,
            );

            let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

            (texture, texture_view)
        };

        let tile_mask_tex = {
            use image::GenericImageView;
            let texture =
                image::load_from_memory(include_bytes!("../../resources/mask_sheet.png")).unwrap();
            let (width, height) = texture.dimensions();
            assert_eq!(width, 64);
            assert_eq!(height, 64);
            let pixels = texture.into_luma8();
            assert_eq!(pixels.len(), 64 * 64);

            let texture = device.create_texture_with_data(
                &queue,
                &wgpu::TextureDescriptor {
                    label: Some("Tile Sheet Texture"),
                    size: wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::R8Uint,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                },
                wgpu::util::TextureDataOrder::LayerMajor,
                &pixels,
            );

            let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

            (texture, texture_view)
        };

        // Generic generic_sampler used for all textures.
        let generic_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Generic index buffer.
        let quad_ibo = {
            #[rustfmt::skip]
            let ibo_data: Vec<u16> = (0..13107)
                .into_iter()
                .flat_map(|i| [i * 4 + 0, i * 4 + 3, i * 4 + 1, i * 4 + 2, u16::MAX])
                .collect();
            assert_eq!(ibo_data.len(), 65535);

            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(&ibo_data),
                usage: wgpu::BufferUsages::INDEX,
            });

            buffer
        };

        // Create camera buffer.
        let view_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("View Uniform"),
            size: std::mem::size_of::<ViewUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Misc bind group.
        let misc_bind_group = {
            let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Misc Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                        count: None,
                    },
                ],
            });

            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Misc Bind Group"),
                layout: &layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: view_uniform.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&generic_sampler),
                    },
                ],
            });

            (group, layout)
        };

        // Create tile rendering pipeline.
        let (tile_pipeline, tile_bind_group) = {
            // Shader.
            let shader = device.create_shader_module(wgpu::include_wgsl!("shaders/tile.wgsl"));

            // Bind group.
            let bind_group = {
                let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                multisampled: false,
                                view_dimension: wgpu::TextureViewDimension::D2,
                                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                multisampled: false,
                                view_dimension: wgpu::TextureViewDimension::D2,
                                sample_type: wgpu::TextureSampleType::Uint,
                            },
                            count: None,
                        },
                    ],
                    label: Some("Tile Bind Group Layout"),
                });

                let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&tile_sprite_tex.1),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(&tile_mask_tex.1),
                        },
                    ],
                    label: Some("Tile Bind Group"),
                });

                (group, layout)
            };

            // Pipeline layout.
            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&misc_bind_group.1, &bind_group.1],
                push_constant_ranges: &[],
            });

            // Render pipeline.
            let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Tile Render Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<TileVertexInput>() as _,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &TileVertexInput::ATTRIB,
                    }],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleStrip,
                    strip_index_format: Some(wgpu::IndexFormat::Uint16),
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState {
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
            format,

            quad_ibo,

            misc_bind_group: misc_bind_group.0,
            view_uniform,
            generic_sampler,

            tile_sprite_tex,
            tile_mask_tex,

            tile_pipeline,
            tile_bind_group: tile_bind_group.0,
        }
    }

    pub fn render(&mut self, _ts: u64, game_frame: GameFrame) {
        // Calculate view matrix.
        let view = {
            let view = Mat3::identity();
            let view = view
                * scaling2d(&Vec2::new(
                    2. / game_frame.viewport_w,
                    -2. / game_frame.viewport_h,
                ));
            let view = view
                * translation2d(&Vec2::new(
                    -game_frame.viewport_x - game_frame.viewport_w / 2.,
                    -game_frame.viewport_y - game_frame.viewport_h / 2.,
                ));
            view
        };

        // Calculate tile vertex data.
        let max_tiles = (game_frame.tiles_w - 2) * (game_frame.tiles_h - 2);
        let mut fg_vertex_tiles = Vec::with_capacity(max_tiles);
        if max_tiles > 0 {
            // Calculate tile data and upload to GPU.
            let tile_texture_properties_lookup = &crate::shared::TILE_TEXTURE_PROPERTIES;
            let stride = game_frame.tiles_w;
            'calc_tiles: {
                for y in 1..game_frame.tiles_h - 1 {
                    'x: for x in 1..game_frame.tiles_w - 1 {
                        let index = x + y * game_frame.tiles_w;

                        // Fill FG.
                        'skip_fg: {
                            let tile_texture_properties =
                                tile_texture_properties_lookup[game_frame.fg_tiles[index] as usize];

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
                            let x = 16. * (x + game_frame.tiles_x) as f32;
                            let y = 16. * (y + game_frame.tiles_y) as f32;
                            let z = depth as f32;

                            // Calculate mask UV.
                            #[rustfmt::skip]
                            let mask_u = [ index - stride, index - stride + 1, index + 1, index + stride + 1 ]
                                .into_iter()
                                .rev()
                                .map(|index| game_frame.fg_tiles[index])
                                .map(|tile| tile_texture_properties_lookup[tile as usize].depth)
                                .map(|dep| (depth > dep) as u8)
                                .reduce(|acc, bit| (acc << 1) | bit)
                                .map(|out| (out << 2) as f32)
                                .unwrap();
                            #[rustfmt::skip]
                            let mask_v  = [index + stride, index + stride - 1, index - 1, index - stride - 1 ]
                                .into_iter()
                                .rev()
                                .map(|index| game_frame.fg_tiles[index])
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
                    }
                }

                break 'calc_tiles;
            };
        }

        // Begin rendering.
        {
            // Upload camera.
            {
                self.queue.write_buffer(
                    &self.view_uniform,
                    0,
                    bytemuck::cast_slice(&[ViewUniform(nalgebra_glm::mat3_to_mat4(&view).into())]),
                );
            }

            // Input buffer.
            let fg_vertex_input =
                self.device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("FG Vertex Buffer"),
                        contents: bytemuck::cast_slice(&fg_vertex_tiles),
                        usage: wgpu::BufferUsages::VERTEX,
                    });

            let output = match self.surface.get_current_texture() {
                Err(wgpu::SurfaceError::Timeout) => return,
                Ok(output) => output,
                _ => panic!(),
            };

            let view = output
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_index_buffer(self.quad_ibo.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.set_bind_group(0, &self.misc_bind_group, &[]);
            render_pass.set_bind_group(1, &self.tile_bind_group, &[]);

            // FG Tile Rendering.
            render_pass.set_pipeline(&self.tile_pipeline);
            render_pass.set_vertex_buffer(0, fg_vertex_input.slice(..));
            render_pass.draw_indexed(0..fg_vertex_tiles.len() as u32 / 4 * 5, 0, 0..1);

            drop(render_pass);
            self.queue.submit(std::iter::once(encoder.finish()));
            output.present();
        }
    }
}
