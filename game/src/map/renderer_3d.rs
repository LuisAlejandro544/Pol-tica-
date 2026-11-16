//! Propósito del archivo:
//! Renderizador 3D táctico en tiempo real (`wgpu 25` + `egui-wgpu 0.32`) para
//! `latinoamerica_3d.glb`. Dibuja el continente entero con relieve andino, batimetría
//! oceánica, fronteras, ríos y capitales en un único draw call con depth-buffer,
//! iluminación direccional, modos de mapa dinámicos y resaltado de país/provincia.

use crate::map::glb_loader::Vertex3D;
use bytemuck::{Pod, Zeroable};
use eframe::egui_wgpu::{self, wgpu};
use glam::{Mat4, Vec3, Vec4};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Map3dCamera {
    pub target: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
}

impl Default for Map3dCamera {
    fn default() -> Self {
        Self {
            target: [-2.0, 0.5, -2.0],
            yaw: 0.0,
            pitch: 0.98, // ~56 grados: vista táctica 3D inclinada sobre Latinoamérica
            distance: 58.0,
        }
    }
}

impl Map3dCamera {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn set_tactical_3d(&mut self) {
        self.pitch = 0.94;
    }

    pub fn set_top_down(&mut self) {
        self.pitch = 1.46;
        self.yaw = 0.0;
    }

    pub fn is_top_down(&self) -> bool {
        self.pitch > 1.30
    }

    pub fn eye_position(&self) -> Vec3 {
        let t = Vec3::from(self.target);
        let cp = self.pitch.cos();
        let sp = self.pitch.sin();
        let sy = self.yaw.sin();
        let cy = self.yaw.cos();
        t + Vec3::new(self.distance * cp * sy, self.distance * sp, self.distance * cp * cy)
    }

    pub fn view_proj(&self, aspect: f32) -> Mat4 {
        let aspect = aspect.max(0.1);
        let eye = self.eye_position();
        let target = Vec3::from(self.target);
        let view = Mat4::look_at_rh(eye, target, Vec3::Y);
        let proj = Mat4::perspective_rh(42.0_f32.to_radians(), aspect, 0.4, 380.0);
        proj * view
    }

    pub fn pan_by_screen_delta(&mut self, delta: egui::Vec2, rect_height: f32) {
        let scale = (self.distance * 0.95) / rect_height.max(120.0);
        let sy = self.yaw.sin();
        let cy = self.yaw.cos();
        let right_xz = Vec3::new(cy, 0.0, -sy);
        let up_xz = Vec3::new(-sy, 0.0, -cy);

        let mut t = Vec3::from(self.target);
        t -= right_xz * (delta.x * scale);
        t -= up_xz * (delta.y * scale);

        t.x = t.x.clamp(-46.0, 40.0);
        t.z = t.z.clamp(-46.0, 45.0);
        self.target = t.to_array();
    }

    pub fn zoom_by_factor(&mut self, factor: f32) {
        self.distance = (self.distance / factor).clamp(7.5, 105.0);
    }

    pub fn orbit_by_delta(&mut self, delta: egui::Vec2) {
        self.yaw = (self.yaw - delta.x * 0.0065).clamp(-1.35, 1.35);
        self.pitch = (self.pitch + delta.y * 0.0060).clamp(0.38, 1.48);
    }

    pub fn focus_point(&mut self, pt: Vec3, distance: f32) {
        self.target = [pt.x.clamp(-44.0, 38.0), 0.5, pt.z.clamp(-44.0, 43.0)];
        self.distance = distance.clamp(9.0, 85.0);
    }

    /// Proyecta un toque en coordenadas de pantalla dentro de `rect` hacia un rayo 3D en mundo.
    pub fn screen_to_ray(&self, pos: egui::Pos2, rect: egui::Rect) -> (Vec3, Vec3) {
        let aspect = (rect.width() / rect.height().max(1.0)).max(0.1);
        let ndc_x = ((pos.x - rect.left()) / rect.width().max(1.0)) * 2.0 - 1.0;
        let ndc_y = 1.0 - ((pos.y - rect.top()) / rect.height().max(1.0)) * 2.0;

        let inv_vp = self.view_proj(aspect).inverse();
        let near_clip = inv_vp * Vec4::new(ndc_x, ndc_y, 0.0, 1.0);
        let far_clip = inv_vp * Vec4::new(ndc_x, ndc_y, 1.0, 1.0);

        let near = near_clip.truncate() / near_clip.w.max(1e-6);
        let far = far_clip.truncate() / far_clip.w.max(1e-6);
        (near, (far - near).normalize_or_zero())
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct MapUniforms {
    pub view_proj: [[f32; 4]; 4],
    pub camera_pos: [f32; 4],
    /// [selected_country_slot, player_country_slot, selected_province_slot, map_mode_id]
    pub flags: [i32; 4],
    /// Colores lineales por slot de país (hasta 32) usados en modos PIB / Inflación / Diplomacia
    pub country_palette: [[f32; 4]; 32],
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct BlitUniforms {
    /// 1.0 si la superficie destino NO es sRGB (requiere codificación gamma manual)
    encode_srgb: f32,
    _pad: [f32; 3],
}

pub struct Map3dPaintCallback {
    pub vertices: Arc<Vec<Vertex3D>>,
    pub indices: Arc<Vec<u32>>,
    pub uniforms: MapUniforms,
    pub target_format: wgpu::TextureFormat,
    pub viewport_px: [u32; 2],
    pub gpu_3d_failed: Arc<AtomicBool>,
}

enum Map3dGpuState {
    Ready(Map3dGpuResources),
    Failed,
}

struct Map3dGpuResources {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    map_pipeline: wgpu::RenderPipeline,

    offscreen_size: [u32; 2],
    color_texture: wgpu::Texture,
    color_view: wgpu::TextureView,
    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,

    blit_sampler: wgpu::Sampler,
    blit_bind_group_layout: wgpu::BindGroupLayout,
    blit_bind_group: wgpu::BindGroup,
    blit_uniform_buffer: wgpu::Buffer,
    blit_shader: wgpu::ShaderModule,
    blit_pipeline_layout: wgpu::PipelineLayout,
    blit_pipeline: wgpu::RenderPipeline,
    target_format: wgpu::TextureFormat,
}

impl Map3dGpuResources {
    fn try_new(
        device: &wgpu::Device,
        vertices: &[Vertex3D],
        indices: &[u32],
        initial_size: [u32; 2],
        target_format: wgpu::TextureFormat,
    ) -> Result<Self, String> {
        if vertices.is_empty() || indices.is_empty() {
            return Err("Geometría 3D vacía para wgpu".to_string());
        }
        let limits = device.limits();
        let vb_bytes = std::mem::size_of_val(vertices) as u64;
        let ib_bytes = std::mem::size_of_val(indices) as u64;
        if vb_bytes > limits.max_buffer_size || ib_bytes > limits.max_buffer_size {
            return Err(format!(
                "Buffers 3D exceden max_buffer_size ({})",
                limits.max_buffer_size
            ));
        }

        let max_tex = limits.max_texture_dimension_2d.min(2048).max(64);
        let safe_size = [
            initial_size[0].clamp(64, max_tex),
            initial_size[1].clamp(64, max_tex),
        ];

        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            Self::new(device, vertices, indices, safe_size, target_format)
        }))
        .map_err(|_| "Fallo inicializando pipeline wgpu 3D en esta GPU móvil".to_string())
    }

    fn new(
        device: &wgpu::Device,
        vertices: &[Vertex3D],
        indices: &[u32],
        initial_size: [u32; 2],
        target_format: wgpu::TextureFormat,
    ) -> Self {
        let vertex_buffer = create_buffer_with_data(
            device,
            "latam_3d_vb",
            wgpu::BufferUsages::VERTEX,
            bytemuck::cast_slice(vertices),
        );
        let index_buffer = create_buffer_with_data(
            device,
            "latam_3d_ib",
            wgpu::BufferUsages::INDEX,
            bytemuck::cast_slice(indices),
        );

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("latam_3d_uniforms"),
            size: std::mem::size_of::<MapUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let uniform_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("latam_3d_bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("latam_3d_bg"),
            layout: &uniform_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let shader_3d = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("latam_3d_shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER_3D_WGSL.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("latam_3d_pipeline_layout"),
            bind_group_layouts: &[&uniform_bgl],
            push_constant_ranges: &[],
        });

        let vertex_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex3D>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 12,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 24,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 40,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Sint32x2,
                },
            ],
        };

        let map_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("latam_3d_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_3d,
                entry_point: Some("vs_main"),
                buffers: &[vertex_layout],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader_3d,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            multiview: None,
            cache: None,
        });

        let w = initial_size[0].max(64);
        let h = initial_size[1].max(64);
        let (color_texture, color_view, depth_texture, depth_view) =
            create_offscreen_targets(device, w, h);

        let blit_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("latam_blit_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let blit_uniform_data = BlitUniforms {
            encode_srgb: if target_format.is_srgb() { 0.0 } else { 1.0 },
            _pad: [0.0; 3],
        };
        let blit_uniform_buffer = create_buffer_with_data(
            device,
            "latam_blit_uniform",
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            bytemuck::bytes_of(&blit_uniform_data),
        );

        let blit_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("latam_blit_bgl"),
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
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let blit_bind_group = create_blit_bind_group(
            device,
            &blit_bind_group_layout,
            &color_view,
            &blit_sampler,
            &blit_uniform_buffer,
        );

        let blit_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("latam_blit_shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER_BLIT_WGSL.into()),
        });

        let blit_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("latam_blit_pl"),
            bind_group_layouts: &[&blit_bind_group_layout],
            push_constant_ranges: &[],
        });

        let blit_pipeline =
            create_blit_pipeline(device, &blit_pipeline_layout, &blit_shader, target_format);

        Self {
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
            uniform_buffer,
            uniform_bind_group,
            map_pipeline,
            offscreen_size: [w, h],
            color_texture,
            color_view,
            depth_texture,
            depth_view,
            blit_sampler,
            blit_bind_group_layout,
            blit_bind_group,
            blit_uniform_buffer,
            blit_shader,
            blit_pipeline_layout,
            blit_pipeline,
            target_format,
        }
    }

    fn sync_target_format(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        new_format: wgpu::TextureFormat,
    ) -> Result<(), String> {
        if self.target_format == new_format {
            return Ok(());
        }
        let blit_uniform_data = BlitUniforms {
            encode_srgb: if new_format.is_srgb() { 0.0 } else { 1.0 },
            _pad: [0.0; 3],
        };
        queue.write_buffer(
            &self.blit_uniform_buffer,
            0,
            bytemuck::bytes_of(&blit_uniform_data),
        );
        let new_pipeline = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            create_blit_pipeline(
                device,
                &self.blit_pipeline_layout,
                &self.blit_shader,
                new_format,
            )
        }))
        .map_err(|_| "No se pudo adaptar blit_pipeline al formato de superficie".to_string())?;
        self.blit_pipeline = new_pipeline;
        self.target_format = new_format;
        Ok(())
    }

    fn ensure_size(&mut self, device: &wgpu::Device, width: u32, height: u32) -> Result<(), String> {
        let max_dim = device.limits().max_texture_dimension_2d.min(2048).max(64);
        let w = width.clamp(64, max_dim);
        let h = height.clamp(64, max_dim);
        if self.offscreen_size == [w, h] {
            return Ok(());
        }
        let targets = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            create_offscreen_targets(device, w, h)
        }))
        .map_err(|_| "No se pudieron crear texturas offscreen 3D".to_string())?;
        self.color_texture = targets.0;
        self.color_view = targets.1;
        self.depth_texture = targets.2;
        self.depth_view = targets.3;
        self.offscreen_size = [w, h];
        self.blit_bind_group = create_blit_bind_group(
            device,
            &self.blit_bind_group_layout,
            &self.color_view,
            &self.blit_sampler,
            &self.blit_uniform_buffer,
        );
        Ok(())
    }
}

impl egui_wgpu::CallbackTrait for Map3dPaintCallback {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen_descriptor: &egui_wgpu::ScreenDescriptor,
        egui_encoder: &mut wgpu::CommandEncoder,
        callback_resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let max_dim = device.limits().max_texture_dimension_2d.min(2048).max(64);
        let target_w = self.viewport_px[0].clamp(64, max_dim);
        let target_h = self.viewport_px[1].clamp(64, max_dim);

        let gpu_state = callback_resources
            .entry::<Map3dGpuState>()
            .or_insert_with(|| {
                match Map3dGpuResources::try_new(
                    device,
                    &self.vertices,
                    &self.indices,
                    [target_w, target_h],
                    self.target_format,
                ) {
                    Ok(res) => Map3dGpuState::Ready(res),
                    Err(err) => {
                        log::error!("Desactivando mapa 3D por error de GPU: {err}");
                        self.gpu_3d_failed.store(true, Ordering::Relaxed);
                        Map3dGpuState::Failed
                    }
                }
            });

        let Map3dGpuState::Ready(res) = gpu_state else {
            self.gpu_3d_failed.store(true, Ordering::Relaxed);
            return Vec::new();
        };

        if let Err(err) = res.sync_target_format(device, queue, self.target_format) {
            log::error!("Error sincronizando target_format 3D: {err}");
            *gpu_state = Map3dGpuState::Failed;
            self.gpu_3d_failed.store(true, Ordering::Relaxed);
            return Vec::new();
        }

        if let Err(err) = res.ensure_size(device, target_w, target_h) {
            log::error!("Error redimensionando offscreen 3D: {err}");
            *gpu_state = Map3dGpuState::Failed;
            self.gpu_3d_failed.store(true, Ordering::Relaxed);
            return Vec::new();
        }

        if res.index_count == 0 {
            return Vec::new();
        }

        queue.write_buffer(&res.uniform_buffer, 0, bytemuck::bytes_of(&self.uniforms));

        {
            let mut pass = egui_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("latam_3d_offscreen_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &res.color_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.012,
                            g: 0.032,
                            b: 0.068,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &res.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            pass.set_pipeline(&res.map_pipeline);
            pass.set_bind_group(0, &res.uniform_bind_group, &[]);
            pass.set_vertex_buffer(0, res.vertex_buffer.slice(..));
            pass.set_index_buffer(res.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..res.index_count, 0, 0..1);
        }

        Vec::new()
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        callback_resources: &egui_wgpu::CallbackResources,
    ) {
        if let Some(Map3dGpuState::Ready(res)) = callback_resources.get::<Map3dGpuState>() {
            if res.target_format == self.target_format && res.index_count > 0 {
                render_pass.set_pipeline(&res.blit_pipeline);
                render_pass.set_bind_group(0, &res.blit_bind_group, &[]);
                render_pass.draw(0..3, 0..1);
            }
        }
    }
}

fn create_blit_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    target_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("latam_blit_pipeline"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: target_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        multiview: None,
        cache: None,
    })
}

fn create_buffer_with_data(
    device: &wgpu::Device,
    label: &str,
    usage: wgpu::BufferUsages,
    bytes: &[u8],
) -> wgpu::Buffer {
    let size = ((bytes.len().max(16) as u64) + 3) & !3;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size,
        usage,
        mapped_at_creation: true,
    });
    if !bytes.is_empty() {
        let mut mapped = buffer.slice(..bytes.len() as u64).get_mapped_range_mut();
        mapped.copy_from_slice(bytes);
    }
    buffer.unmap();
    buffer
}

fn create_offscreen_targets(
    device: &wgpu::Device,
    width: u32,
    height: u32,
) -> (
    wgpu::Texture,
    wgpu::TextureView,
    wgpu::Texture,
    wgpu::TextureView,
) {
    let color_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("latam_3d_color_tex"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let color_view = color_texture.create_view(&wgpu::TextureViewDescriptor::default());

    let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("latam_3d_depth_tex"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

    (color_texture, color_view, depth_texture, depth_view)
}

fn create_blit_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    color_view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
    uniform_buffer: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("latam_blit_bg"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(color_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: uniform_buffer.as_entire_binding(),
            },
        ],
    })
}

/// Convierte un color sRGB [0..255] a Linear RGB [0..1] para shaders/paleta 3D.
pub fn srgb_u8_to_linear(rgb: [u8; 3]) -> [f32; 3] {
    let conv = |c: u8| -> f32 {
        let u = (c as f32) / 255.0;
        if u <= 0.04045 {
            u / 12.92
        } else {
            ((u + 0.055) / 1.055).powf(2.4)
        }
    };
    [conv(rgb[0]), conv(rgb[1]), conv(rgb[2])]
}

const SHADER_3D_WGSL: &str = r#"
struct MapUniforms {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    flags: vec4<i32>, // x: sel_country, y: player_country, z: sel_prov, w: map_mode
    country_palette: array<vec4<f32>, 32>,
};

@group(0) @binding(0)
var<uniform> u: MapUniforms;

struct VsIn {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) meta: vec2<i32>,
};

struct VsOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) @interpolate(flat) meta: vec2<i32>,
};

@vertex
fn vs_main(v: VsIn) -> VsOut {
    var out: VsOut;
    out.clip_pos = u.view_proj * vec4<f32>(v.pos, 1.0);
    out.world_pos = v.pos;
    out.world_normal = v.normal;
    out.color = v.color;
    out.meta = v.meta;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    var base = in.color.rgb;
    let c_slot = in.meta.x;
    let p_slot = in.meta.y;

    if (c_slot >= 0 && c_slot < 32) {
        // Modo distinto a Político (1=PIB, 2=Inflación, 3=Diplomacia)
        if (u.flags.w != 0) {
            let mode_col = u.country_palette[c_slot].rgb;
            let lum = clamp(dot(base, vec3<f32>(0.299, 0.587, 0.114)) * 1.45, 0.62, 1.15);
            base = mix(base, mode_col * lum, 0.88);
        }

        // Resaltado sutil del país del jugador (cian) y fuerte del país/provincia seleccionados (oro)
        if (p_slot == u.flags.z && u.flags.z >= 0) {
            base = mix(base, vec3<f32>(0.92, 0.68, 0.14), 0.45);
        } else if (c_slot == u.flags.x && u.flags.x >= 0) {
            base = mix(base, vec3<f32>(0.85, 0.62, 0.12), 0.24);
        } else if (c_slot == u.flags.y && u.flags.y >= 0) {
            base = mix(base, vec3<f32>(0.10, 0.56, 0.88), 0.16);
        }
    }

    let n_len = length(in.world_normal);
    let n = select(vec3<f32>(0.0, 1.0, 0.0), in.world_normal / n_len, n_len > 0.00001);
    let sun_dir = normalize(vec3<f32>(0.42, 0.84, 0.34));
    let rim_dir = normalize(vec3<f32>(-0.45, 0.55, -0.50));

    let ndl = max(dot(n, sun_dir), 0.0);
    let rim = max(dot(n, rim_dir), 0.0) * 0.22;
    let hemi = 0.5 + 0.5 * n.y;
    let ambient = mix(vec3<f32>(0.20, 0.24, 0.30), vec3<f32>(0.46, 0.50, 0.56), hemi);

    let lit = base * (ambient + vec3<f32>(0.86, 0.83, 0.78) * ndl + vec3<f32>(0.40, 0.52, 0.68) * rim);
    return vec4<f32>(lit, 1.0);
}
"#;

const SHADER_BLIT_WGSL: &str = r#"
struct BlitUniforms {
    encode_srgb: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
};

@group(0) @binding(0) var tex_color: texture_2d<f32>;
@group(0) @binding(1) var tex_sampler: sampler;
@group(0) @binding(2) var<uniform> blit_u: BlitUniforms;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vid: u32) -> VsOut {
    let x = f32((vid << 1u) & 2u);
    let y = f32(vid & 2u);
    var out: VsOut;
    out.pos = vec4<f32>(x * 2.0 - 1.0, y * 2.0 - 1.0, 0.0, 1.0);
    out.uv = vec2<f32>(x, 1.0 - y);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    var col = textureSample(tex_color, tex_sampler, in.uv);
    if (blit_u.encode_srgb > 0.5) {
        let rgb = pow(max(col.rgb, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.2));
        return vec4<f32>(rgb, col.a);
    }
    return col;
}
"#;
