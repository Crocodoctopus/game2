use crate::client::GameFrame;
use image::GenericImageView;
use nalgebra_glm::*;
use std::path::Path;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct TileVertex {
    tile_xyz: [f32; 3],
    tile_uv: [f32; 2],
    mask_uv: [f32; 2],
}

impl TileVertex {
    // NOTE: `wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2];` is also valid
    const VAO: &'static [wgpu::VertexAttribute] = 
        &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x2];
    /*
    const VAO: &'static [wgpu::VertexAttribute] = &[
        wgpu::VertexAttribute {
            offset: 0,
            shader_location: 0,
            format: wgpu::VertexFormat::Float32x2,
        },
        wgpu::VertexAttribute {
            offset: std::mem::offset_of!(TileVertex, mask_uv) as wgpu::BufferAddress,
            shader_location: 1,
            format: wgpu::VertexFormat::Float32x2,
        },
    ];
    */

    const fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<TileVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: Self::VAO,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct LightVertex {
    light_xy: [f32; 2],
}

impl LightVertex {
    const VAO: &'static [wgpu::VertexAttribute] = 
        &wgpu::vertex_attr_array![0 => Float32x2];

    const fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<TileVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: Self::VAO,
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_matrix: [[f32; 4]; 4],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct TileUniform {
    mul_rgb: [f32; 3],
}

pub struct GameRenderStateWgpu {
    instance: wgpu::Instance,
    // pure unsafe hackery
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: (i32, i32),

    tile_render_pipeline: wgpu::RenderPipeline,
    light_render_pipeline: wgpu::RenderPipeline,

    fg_vertex_buffer: wgpu::Buffer,
    bg_vertex_buffer: wgpu::Buffer,
    light_vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,

    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,

    tile_uniform: TileUniform,
    tile_uniform_buffer: wgpu::Buffer,
    tile_uniform_bind_group: wgpu::BindGroup,

    tile_texture_bind_group: wgpu::BindGroup,
    
    light_texture_bind_group: wgpu::BindGroup,
    light_texture: wgpu::Texture,
}

impl GameRenderStateWgpu {
    pub fn new(_root: &'static Path, window: &mut crate::window::Window) -> Self {
        let size = window.window.get_size();
        let scale = window.window.get_content_scale();
        let size = ((size.0 as f32 * scale.0) as i32, (size.1 as f32 * scale.1) as i32);

        // The instance is a handle to our GPU
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: Default::default(),
            flags: wgpu::InstanceFlags::empty(),
            gles_minor_version: Default::default(),
        });
        
        // # Safety
        //
        // The surface needs to live as long as the window that created it.
        // State owns the window so this should be safe.
        let surface = unsafe { instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::from_window(&window.window).unwrap()) }.unwrap();

        //let adapter = instance.enumerate_adapters(wgpu::Backends::all()).first().unwrap();

        let adapter = pollster::block_on( async {
            instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            },
        ).await.unwrap() });

        let (device, queue) = pollster::block_on(async { adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
            },
            None, // Trace path
        ).await.unwrap() });
 
        let surface_caps = surface.get_capabilities(&adapter);
        // Shader code in this tutorial assumes an sRGB surface texture. Using a different
        // one will result all the colors coming out darker. If you want to support non
        // sRGB surfaces, you'll need to account for that when drawing to the frame.
        let surface_format = surface_caps.formats.iter()
            .copied()
            //.filter(|f| f.describe().srgb)
            .next()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.0 as u32,
            height: size.1 as u32,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let tile_sheet = {
            // Load 
            let image = image::load_from_memory(include_bytes!("../../resources/tile_sheet.png")).unwrap();
            let data = image.as_rgba8().unwrap();
            let (width, height) = image.dimensions();
            let texture_size = wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            };
            let texture = device.create_texture(
                &wgpu::TextureDescriptor {
                    // All textures are stored as 3D, we represent our 2D texture
                    // by setting depth to 1.
                    size: texture_size,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    // Most images are stored using sRGB, so we need to reflect that here.
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    // TEXTURE_BINDING tells wgpu that we want to use this texture in shaders
                    // COPY_DST means that we want to copy data to this texture
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    label: Some("tile_sheet"),
                    // This is the same as with the SurfaceConfig. It
                    // specifies what texture formats can be used to
                    // create TextureViews for this texture. The base
                    // texture format (Rgba8UnormSrgb in this case) is
                    // always supported. Note that using a different
                    // texture format is not supported on the WebGL2
                    // backend.
                    view_formats: &[],
                }
            );
            queue.write_texture(
                // Tells wgpu where to copy the pixel data
                wgpu::ImageCopyTexture {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                // The actual pixel data
                &data,
                // The layout of the texture
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * width),
                    rows_per_image: Some(height),
                },
                texture_size,
            );
            texture
        };

        let mask_sheet = {
            // Load 
            let image = image::load_from_memory(include_bytes!("../../resources/mask_sheet.png")).unwrap();
            let (width, height) = image.dimensions();
            let data = image.into_luma8();
            let texture_size = wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            };
            let texture = device.create_texture(
                &wgpu::TextureDescriptor {
                    size: texture_size,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::R8Unorm,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    label: Some("mask_sheet"),
                    view_formats: &[],
                }
            );
            queue.write_texture(
                // Tells wgpu where to copy the pixel data
                wgpu::ImageCopyTexture {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                // The actual pixel data
                &data,
                // The layout of the texture
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(width),
                    rows_per_image: Some(height),
                },
                texture_size,
            );
            texture
        };

        let light_texture = {
            // Load 
            // No idea, it's dynamic
            let (width, height) = (256, 256);
            let texture_size = wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            };
            let texture = device.create_texture(
                &wgpu::TextureDescriptor {
                    size: texture_size,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8Uint,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    label: Some("light_texture"),
                    view_formats: &[],
                }
            );

            texture
        };

        let tile_sheet_texture_view = tile_sheet.create_view(&wgpu::TextureViewDescriptor::default());
        let mask_sheet_texture_view = mask_sheet.create_view(&wgpu::TextureViewDescriptor::default());
        let light_texture_view = light_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let texture_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        // This should match the filterable field of the
                        // corresponding Texture entry above.
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        let tile_texture_bind_group = device.create_bind_group(
                &wgpu::BindGroupDescriptor {
                    layout: &texture_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&tile_sheet_texture_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(&mask_sheet_texture_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::Sampler(&texture_sampler),
                        }
                    ],
                    label: Some("tile_sheet_bind_group"),
                }
            );

        let light_texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Uint,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        // This should match the filterable field of the
                        // corresponding Texture entry above.
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("light_texture_bind_group_layout"),
            });

        let light_texture_bind_group = device.create_bind_group(
                &wgpu::BindGroupDescriptor {
                    layout: &light_texture_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&light_texture_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::Sampler(&texture_sampler),
                        }
                    ],
                    label: Some("light_texture_bind_group"),
                }
            );
            
        let tile_shader = device.create_shader_module(wgpu::include_wgsl!("shaders/tile.wgsl"));
        let light_shader = device.create_shader_module(wgpu::include_wgsl!("shaders/light.wgsl"));

        let camera_uniform = CameraUniform {
            view_matrix: Mat4::identity().into(),
        };

        let camera_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Camera Buffer"),
                contents: bytemuck::cast_slice(&[camera_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );

        let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                }
            ],
            label: Some("camera_bind_group_layout"),
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                }
            ],
            label: Some("camera_bind_group"),
        });

        let tile_uniform = TileUniform {
            mul_rgb: [1.0, 1.0, 1.0],
        };

        let tile_uniform_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Tile Uniform Buffer"),
                contents: bytemuck::cast_slice(&[tile_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );

        let tile_uniform_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
            label: Some("tile_uniform_bind_group_layout"),
        });

        let tile_uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &tile_uniform_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: tile_uniform_buffer.as_entire_binding(),
                }
            ],
            label: Some("tile_uniform_bind_group"),
        });

        let tile_render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &camera_bind_group_layout,
                    &texture_bind_group_layout,
                    &tile_uniform_bind_group_layout,
                ],
                push_constant_ranges: &[],
        });

        let light_render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Light Render Pipeline Layout"),
                bind_group_layouts: &[
                    &camera_bind_group_layout,
                    &light_texture_bind_group_layout,
                ],
                push_constant_ranges: &[],
        });

        let tile_render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&tile_render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &tile_shader,
                entry_point: "vs_main",
                buffers: &[
                    TileVertex::desc(),
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &tile_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: Some(wgpu::IndexFormat::Uint16),
                front_face: wgpu::FrontFace::Cw,
                cull_mode: None,//Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
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

        let light_render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Light Render Pipeline"),
            layout: Some(&light_render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &light_shader,
                entry_point: "vs_main",
                buffers: &[
                    TileVertex::desc(),
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &light_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: Some(wgpu::IndexFormat::Uint16),
                front_face: wgpu::FrontFace::Cw,
                cull_mode: None,//Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
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

        let vertex_data: Vec<TileVertex> = vec![];
        let light_vertex_data: Vec<LightVertex> = vec![];

        #[rustfmt::skip]
        let ibo_data: Vec<u16> = (0..13107)
            .into_iter()
            .flat_map(|i| [i * 4 + 0, i * 4 + 3, i * 4 + 1, i * 4 + 2, u16::MAX])
            //.flat_map(|i| [4 * i + 0, 4 * i + 1, 4 * i + 2, 4 * i + 3, u16::MAX])
            .collect();
        assert_eq!(ibo_data.len(), 65535);

        let fg_vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("FG Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertex_data),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );
        let bg_vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("BG Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertex_data),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );
        let light_vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Light Vertex Buffer"),
                contents: bytemuck::cast_slice(&light_vertex_data),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(&ibo_data),
                usage: wgpu::BufferUsages::INDEX,
            }
        );
        let num_indices = ibo_data.len() as u32;


        Self {
            instance,
            surface,
            device,
            queue,
            config,
            size,
            tile_render_pipeline,
            light_render_pipeline,
            fg_vertex_buffer,
            bg_vertex_buffer,
            light_vertex_buffer,
            index_buffer,
            num_indices,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            tile_uniform,
            tile_uniform_buffer,
            tile_uniform_bind_group,
            tile_texture_bind_group,
            light_texture_bind_group,
            light_texture,
        }
    }

    pub fn resize(&mut self, new_size: (i32, i32)) {
        if new_size.0 > 0 && new_size.1 > 0 {
            self.size = new_size;
            self.config.width = new_size.0 as u32;
            self.config.height = new_size.1 as u32;
            self.surface.configure(&self.device, &self.config);
        }   
    }

    /// # Safety
    /// - It's not lmao
    pub fn update_surface(&mut self, window: &mut crate::window::Window) {
        unsafe { self.instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::from_window(&window.window).unwrap()) }.unwrap();
    }

    pub fn render(&mut self, _ts: u64, game_frame: GameFrame) {
        // View mat3.
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
            nalgebra_glm::mat3_to_mat4(&view)
        };
        self.camera_uniform.view_matrix = view.into();
        self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[self.camera_uniform]));

        // Render tiles.
        let max_tiles = (game_frame.tiles_w - 2) * (game_frame.tiles_h - 2);
        let mut fg_vertex_tiles = Vec::with_capacity(max_tiles);
        let mut bg_vertex_tiles = Vec::with_capacity(max_tiles);
        'skip: {
            if max_tiles == 0 {
                break 'skip;
            }

            // Calculate tile data and upload to GPU.
            'calc_tiles: {
                for y in 1..game_frame.tiles_h - 1 {
                    'x: for x in 1..game_frame.tiles_w - 1 {
                        let index = x + y * game_frame.tiles_w;

                        // Fill FG.
                        'skip_fg: {
                            let tile = game_frame.fg_tiles[index];
                            if tile == 0 {
                                break 'skip_fg;
                            }

                            // Fill vertex data.
                            let tx = 16. * (x + game_frame.tiles_x) as f32;
                            let ty = 16. * (y + game_frame.tiles_y) as f32;

                            let uv_tx = 16. * tile as f32;

                            let tw = game_frame.tiles_w;
                            let t0 = ((tile > game_frame.fg_tiles[index - tw]) as u8) << 0;
                            let t1 = ((tile > game_frame.fg_tiles[index - tw + 1]) as u8) << 1;
                            let t2 = ((tile > game_frame.fg_tiles[index + 1]) as u8) << 2;
                            let t3 = ((tile > game_frame.fg_tiles[index + tw + 1]) as u8) << 3;
                            let u = ((t0 | t1 | t2 | t3) << 2) as f32;
                            let t4 = ((tile > game_frame.fg_tiles[index + tw]) as u8) << 0;
                            let t5 = ((tile > game_frame.fg_tiles[index + tw - 1]) as u8) << 1;
                            let t6 = ((tile > game_frame.fg_tiles[index - 1]) as u8) << 2;
                            let t7 = ((tile > game_frame.fg_tiles[index - tw - 1]) as u8) << 3;
                            let v = ((t4 | t5 | t6 | t7) << 2) as f32;
                            #[rustfmt::skip]
                            fg_vertex_tiles.push(TileVertex {
                                tile_xyz: [tx - 8.,       ty - 8.,       tile as f32],
                                tile_uv:  [uv_tx, 0.],
                                mask_uv:  [u, v],
                            });
                            fg_vertex_tiles.push(TileVertex {
                                tile_xyz: [tx + 16. + 8., ty - 8.,       tile as f32],
                                tile_uv:  [16. + uv_tx, 0.],
                                mask_uv:  [u + 4., v],
                            });
                            fg_vertex_tiles.push(TileVertex {
                                tile_xyz: [tx + 16. + 8., ty + 16. + 8., tile as f32],
                                tile_uv:  [16. + uv_tx, 16.],
                                mask_uv:  [u + 4., v + 4.],
                            });
                            fg_vertex_tiles.push(TileVertex {
                                tile_xyz: [tx - 8.,       ty + 16. + 8., tile as f32],
                                tile_uv:  [0. + uv_tx,  16.],
                                mask_uv:  [u,      v + 4.],
                            });

                            // Skip check bg tile.
                            continue 'x;
                        }

                        // Fill BG.
                        'skip_bg: {
                            let tile = game_frame.bg_tiles[index];
                            if tile == 0 {
                                break 'skip_bg;
                            }

                            // Fill vertex data.
                            let tx = 16. * (x + game_frame.tiles_x) as f32;
                            let ty = 16. * (y + game_frame.tiles_y) as f32;

                            let uv_tx = 16. * tile as f32;
                            let tw = game_frame.tiles_w;
                            let t0 = ((tile > game_frame.bg_tiles[index - tw]) as u8) << 0;
                            let t1 = ((tile > game_frame.bg_tiles[index - tw + 1]) as u8) << 1;
                            let t2 = ((tile > game_frame.bg_tiles[index + 1]) as u8) << 2;
                            let t3 = ((tile > game_frame.bg_tiles[index + tw + 1]) as u8) << 3;
                            let u = ((t0 | t1 | t2 | t3) << 2) as f32;
                            let t4 = ((tile > game_frame.bg_tiles[index + tw]) as u8) << 0;
                            let t5 = ((tile > game_frame.bg_tiles[index + tw - 1]) as u8) << 1;
                            let t6 = ((tile > game_frame.bg_tiles[index - 1]) as u8) << 2;
                            let t7 = ((tile > game_frame.bg_tiles[index - tw - 1]) as u8) << 3;
                            let v = ((t4 | t5 | t6 | t7) << 2) as f32;
                            bg_vertex_tiles.push(TileVertex {
                                tile_xyz: [tx - 8.,       ty - 8.,       tile as f32],
                                tile_uv:  [uv_tx, 0.],
                                mask_uv:  [u, v],
                            });
                            bg_vertex_tiles.push(TileVertex {
                                tile_xyz: [tx + 16. + 8., ty - 8.,       tile as f32],
                                tile_uv:  [16. + uv_tx, 0.],
                                mask_uv:  [u + 4., v],
                            });
                            bg_vertex_tiles.push(TileVertex {
                                tile_xyz: [tx + 16. + 8., ty + 16. + 8., tile as f32],
                                tile_uv:  [16. + uv_tx, 16.],
                                mask_uv:  [u + 4., v + 4.],
                            });
                            bg_vertex_tiles.push(TileVertex {
                                tile_xyz: [tx - 8.,       ty + 16. + 8., tile as f32],
                                tile_uv:  [0. + uv_tx,  16.],
                                mask_uv:  [u,      v + 4.],
                            });
                        }
                    }
                }

                break 'calc_tiles
            };
        }

        // Render lighting.
        let mut light_vertices = vec![];
        {
            // Set up.
            #[rustfmt::skip]
            let _ = unsafe {
                let mut rgba = vec![0u8; game_frame.light_w  * game_frame.light_h * 4];
                for i in 0 .. game_frame.light_w  * game_frame.light_h  {
                    rgba[4 * i + 0] = game_frame.r_channel[i];
                    rgba[4 * i + 1] = game_frame.g_channel[i];
                    rgba[4 * i + 2] = game_frame.b_channel[i];
                    rgba[4 * i + 3] = 255;
                }
                light_vertices.push(
                    LightVertex {
                        light_xy: [game_frame.light_x as f32 * 16., game_frame.light_y as f32 * 16.],
                    },
                );
                light_vertices.push(
                    LightVertex {
                        light_xy: [(game_frame.light_x + game_frame.light_w) as f32 * 16., game_frame.light_y as f32 * 16.],
                    },
                );
                light_vertices.push(
                    LightVertex {
                        light_xy: [(game_frame.light_x + game_frame.light_w) as f32 * 16., (game_frame.light_y + game_frame.light_h) as f32 * 16.],
                    },
                );
                light_vertices.push(
                    LightVertex {
                        light_xy: [game_frame.light_x as f32 * 16., (game_frame.light_y + game_frame.light_h) as f32 * 16.],
                    },
                );
                // TODO:
                let texture_size = wgpu::Extent3d {
                    height: game_frame.light_h as u32,
                    width: game_frame.light_w as u32,
                    depth_or_array_layers: 1,
                };
                self.queue.write_texture(
                    // Tells wgpu where to copy the pixel data
                    wgpu::ImageCopyTexture {
                        texture: &self.light_texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    // The actual pixel data
                    &rgba,
                    // The layout of the texture
                    wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(4 * game_frame.light_w as u32),
                        rows_per_image: Some(game_frame.light_h as u32),
                    },
                    texture_size,
                );
                /*
                gl::BindTexture(gl::TEXTURE_2D, self.light_tex);
                //.gl::PixelStorei(gl::UNPACK_ALIGNMENT, 4);
                gl::TexImage2D(gl::TEXTURE_2D, 0, gl::RGB8UI as GLint, game_frame.light_w as GLint, game_frame.light_h as GLint, 0, gl::RGB_INTEGER, gl::UNSIGNED_BYTE, rgb.as_ptr() as *const GLvoid);
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as GLint);
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as GLint);

                let xy = [
                    Vec2::new(game_frame.light_x as f32 * 16.,                        game_frame.light_y as f32 * 16.                       ),
                    Vec2::new((game_frame.light_x + game_frame.light_w) as f32 * 16., game_frame.light_y as f32 * 16.                       ),
                    Vec2::new((game_frame.light_x + game_frame.light_w) as f32 * 16., (game_frame.light_y + game_frame.light_h) as f32 * 16.),
                    Vec2::new(game_frame.light_x as f32 * 16.,                        (game_frame.light_y + game_frame.light_h) as f32 * 16.), ];
                gl::BindBuffer(gl::ARRAY_BUFFER, self.light_xy);
                gl::BufferData(gl::ARRAY_BUFFER, 4 * 2 * 4, xy.as_ptr() as *const GLvoid, gl::STATIC_DRAW); 
                */
            };

            // Draw.
            /*
            #[rustfmt::skip]
            let _ = unsafe {
                use std::mem::size_of;

                gl::Enable(gl::BLEND);
                gl::BlendEquationSeparate(gl::FUNC_ADD, gl::FUNC_ADD);
                gl::BlendFunc(gl::DST_COLOR, gl::ZERO);

                gl::ActiveTexture(gl::TEXTURE0 + 0);
                gl::BindTexture(gl::TEXTURE_2D, self.light_tex);

                gl::BindVertexArray(self.light_vao);
                gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.quad_ibo);
                gl::BindVertexBuffer(0, self.light_xy, 0, size_of::<Vec2>() as GLint);

                gl::UseProgram(self.light_program);
                gl::Uniform1i(0, 0);
                gl::UniformMatrix3fv(1, 1, gl::FALSE, view.as_ptr());
                gl::DrawElements(gl::TRIANGLE_FAN, 5, gl::UNSIGNED_SHORT, std::ptr::null());

                gl::BindVertexArray(0);
                gl::Disable(gl::BLEND);
            };
            */
        }

        let output = self.surface.get_current_texture().unwrap();//?;

        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        // TODO: test if copying is faster than updating if we care
        self.fg_vertex_buffer.destroy();
        self.fg_vertex_buffer = self.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("FG Vertex Buffer"),
                contents: bytemuck::cast_slice(&fg_vertex_tiles),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );
        self.bg_vertex_buffer.destroy();
        self.bg_vertex_buffer = self.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("BG Vertex Buffer"),
                contents: bytemuck::cast_slice(&bg_vertex_tiles),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );
        self.light_vertex_buffer.destroy();
        self.light_vertex_buffer = self.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Light Vertex Buffer"),
                contents: bytemuck::cast_slice(&light_vertices),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                // This is what @location(0) in the fragment shader targets
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0x15 as f64 / 256.,
                            g: 0x9F as f64 / 256.,
                            b: 0xEA as f64 / 256.,
                            a: 1.0,
                        }),
                        // TODO:
                        store: Default::default(),
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });


            self.tile_uniform.mul_rgb = [1.0, 1.0, 1.0];
            self.queue.write_buffer(&self.tile_uniform_buffer, 0, bytemuck::cast_slice(&[self.tile_uniform]));
            render_pass.set_pipeline(&self.tile_render_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.tile_texture_bind_group, &[]);
            render_pass.set_bind_group(2, &self.tile_uniform_bind_group, &[]);

            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16); // 1.
            assert!(self.num_indices >= fg_vertex_tiles.len() as u32, "Ran out of indices in the IBO!");
            assert!(self.num_indices >= bg_vertex_tiles.len() as u32, "Ran out of indices in the IBO!");

            // background tiles
            self.tile_uniform.mul_rgb = [0.7, 0.7, 0.8];
            self.queue.write_buffer(&self.tile_uniform_buffer, 0, bytemuck::cast_slice(&[self.tile_uniform]));
            render_pass.set_vertex_buffer(0, self.bg_vertex_buffer.slice(..));

            let idx_val = std::cmp::min(self.num_indices, bg_vertex_tiles.len() as u32);
            render_pass.draw_indexed(0..idx_val, 0, 0..1);

            // foreground tiles
            self.tile_uniform.mul_rgb = [1.0, 1.0, 1.0];
            self.queue.write_buffer(&self.tile_uniform_buffer, 0, bytemuck::cast_slice(&[self.tile_uniform]));
            render_pass.set_vertex_buffer(0, self.fg_vertex_buffer.slice(..));

            let idx_val = std::cmp::min(self.num_indices, fg_vertex_tiles.len() as u32);
            render_pass.draw_indexed(0..idx_val, 0, 0..1);

            // Light!
            render_pass.set_pipeline(&self.light_render_pipeline);
            //render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16); // 1.
            render_pass.set_vertex_buffer(0, self.light_vertex_buffer.slice(..));
            render_pass.set_bind_group(1, &self.light_texture_bind_group, &[]);
            // TODO: light texture stuff
            let idx_val = std::cmp::min(self.num_indices, light_vertices.len() as u32);
            render_pass.draw_indexed(0..idx_val, 0, 0..1);

        }


        // submit will accept anything that implements IntoIter
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        //Ok(())
    }
}
