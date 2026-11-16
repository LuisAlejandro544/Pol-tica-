//! Propósito del archivo:
//! Cargador nativo del mapa 3D generado en Blender (`latinoamerica_3d.glb`) usando `gltf` + `glam`.
//! Extrae vértices, normales, colores lineales por vértice o material y jerarquía de nodos,
//! asignando índices de país y provincia para renderizado `wgpu` en un solo draw call y
//! selección táctil exacta 3D mediante raycasting Möller-Trumbore.

use crate::map::svg_loader::ASSETS_DIR;
use bytemuck::{Pod, Zeroable};
use glam::{Mat3, Mat4, Vec3};
use gltf::mesh::util::{ReadColors, ReadIndices};
use std::collections::BTreeMap;
use std::sync::Arc;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex3D {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 4],
    /// meta[0] = country_slot (-1 para entorno/capitales, 0..31 para provincias)
    /// meta[1] = province_slot (-1 si no es provincia)
    pub meta: [f32; 2],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeshKind {
    Environment,
    Province {
        mesh_name: String,
        country_id: String,
    },
    Capital {
        node_name: String,
        country_id: String,
        is_national: bool,
    },
}

#[derive(Debug, Clone)]
pub struct SceneMesh3D {
    pub name: String,
    pub kind: MeshKind,
}

#[derive(Debug, Clone)]
pub struct ProvincePickMesh {
    pub mesh_name: String,
    pub country_id: String,
    pub province_slot: i32,
    pub aabb_min: Vec3,
    pub aabb_max: Vec3,
    pub center: Vec3,
    pub positions: Vec<Vec3>,
    pub indices: Vec<u32>,
}

#[derive(Clone)]
pub struct Latam3dScene {
    pub vertices: Arc<Vec<Vertex3D>>,
    pub indices: Arc<Vec<u32>>,
    pub country_slots: BTreeMap<String, i32>,
    pub province_slots: BTreeMap<String, i32>,
    pub pick_meshes: Vec<ProvincePickMesh>,
    pub bounds_min: Vec3,
    pub bounds_max: Vec3,
    pub default_center: Vec3,
}

impl Latam3dScene {
    /// Intenta cargar `maps/latinoamerica_3d.glb` desde los assets incrustados o rutas locales.
    /// Devuelve `Ok(None)` si el `.glb` aún no está presente para permitir respaldo SVG.
    pub fn load_embedded_or_disk() -> Result<Option<Self>, String> {
        if let Some(file) = ASSETS_DIR.get_file("maps/latinoamerica_3d.glb") {
            return Self::from_glb_bytes(file.contents()).map(Some);
        }

        #[cfg(not(target_os = "android"))]
        for disk_path in [
            "maps/latinoamerica_3d.glb",
            "../maps/latinoamerica_3d.glb",
            "game/assets/maps/latinoamerica_3d.glb",
        ] {
            if let Ok(bytes) = std::fs::read(disk_path) {
                return Self::from_glb_bytes(&bytes).map(Some);
            }
        }

        Ok(None)
    }

    pub fn from_glb_bytes(bytes: &[u8]) -> Result<Self, String> {
        let (document, buffers, _images) =
            gltf::import_slice(bytes).map_err(|e| format!("Error parseando GLB 3D: {e}"))?;

        let mut vertices = Vec::with_capacity(420_000);
        let mut indices = Vec::with_capacity(820_000);
        let mut country_slots: BTreeMap<String, i32> = BTreeMap::new();
        let mut province_slots: BTreeMap<String, i32> = BTreeMap::new();
        let mut pick_meshes = Vec::new();
        let mut bounds_min = Vec3::splat(f32::INFINITY);
        let mut bounds_max = Vec3::splat(f32::NEG_INFINITY);

        let scene = document
            .default_scene()
            .or_else(|| document.scenes().next())
            .ok_or_else(|| "El modelo GLB no contiene escena".to_string())?;

        for node in scene.nodes() {
            visit_node(
                &node,
                Mat4::IDENTITY,
                &buffers,
                &mut vertices,
                &mut indices,
                &mut country_slots,
                &mut province_slots,
                &mut pick_meshes,
                &mut bounds_min,
                &mut bounds_max,
            );
        }

        if vertices.is_empty() || indices.is_empty() {
            return Err("El archivo GLB no contiene geometría válida".to_string());
        }

        let default_center = (bounds_min + bounds_max) * 0.5;
        Ok(Self {
            vertices: Arc::new(vertices),
            indices: Arc::new(indices),
            country_slots,
            province_slots,
            pick_meshes,
            bounds_min,
            bounds_max,
            default_center: Vec3::new(default_center.x, 0.5, default_center.z * 0.15),
        })
    }

    /// Devuelve el centro 3D aproximado de un país promediando sus provincias.
    pub fn country_center(&self, country_id: &str) -> Option<Vec3> {
        let mut sum = Vec3::ZERO;
        let mut count = 0usize;
        for pm in &self.pick_meshes {
            if pm.country_id == country_id {
                sum += pm.center;
                count += 1;
            }
        }
        if count > 0 {
            Some(sum / (count as f32))
        } else {
            None
        }
    }

    /// Raycasting exacto 3D (AABB + Möller-Trumbore) para detectar la provincia bajo el toque.
    pub fn pick_province_at_ray(&self, ray_origin: Vec3, ray_dir: Vec3) -> Option<(&str, &str)> {
        let dir_norm = ray_dir.normalize_or_zero();
        if dir_norm.length_squared() < 0.5 {
            return None;
        }

        let mut best_t = f32::INFINITY;
        let mut best_hit: Option<(&str, &str)> = None;

        for pm in &self.pick_meshes {
            if !ray_intersects_aabb(ray_origin, dir_norm, pm.aabb_min, pm.aabb_max, best_t) {
                continue;
            }
            for tri in pm.indices.chunks_exact(3) {
                let (Some(&v0), Some(&v1), Some(&v2)) = (
                    pm.positions.get(tri[0] as usize),
                    pm.positions.get(tri[1] as usize),
                    pm.positions.get(tri[2] as usize),
                ) else {
                    continue;
                };
                if let Some(t) = ray_triangle_intersect(ray_origin, dir_norm, v0, v1, v2) {
                    if t > 0.0 && t < best_t {
                        best_t = t;
                        best_hit = Some((pm.mesh_name.as_str(), pm.country_id.as_str()));
                    }
                }
            }
        }

        best_hit
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_node(
    node: &gltf::Node<'_>,
    parent_transform: Mat4,
    buffers: &[gltf::buffer::Data],
    out_vertices: &mut Vec<Vertex3D>,
    out_indices: &mut Vec<u32>,
    country_slots: &mut BTreeMap<String, i32>,
    province_slots: &mut BTreeMap<String, i32>,
    pick_meshes: &mut Vec<ProvincePickMesh>,
    bounds_min: &mut Vec3,
    bounds_max: &mut Vec3,
) {
    let local = Mat4::from_cols_array_2d(&node.transform().matrix());
    let world = parent_transform * local;

    let normal_mat = {
        let m3 = Mat3::from_mat4(world);
        if m3.determinant().abs() > 1e-6 {
            m3.inverse().transpose()
        } else {
            Mat3::IDENTITY
        }
    };

    let node_name = node.name().unwrap_or("MESH");
    let kind = classify_node(node_name);

    let (country_slot, province_slot) = match &kind {
        MeshKind::Province {
            mesh_name,
            country_id,
        } => {
            let next_c = country_slots.len() as i32;
            let c_slot = *country_slots
                .entry(country_id.clone())
                .or_insert_with(|| next_c.min(31));
            let next_p = province_slots.len() as i32;
            let p_slot = *province_slots.entry(mesh_name.clone()).or_insert(next_p);
            (c_slot, p_slot)
        }
        _ => (-1, -1),
    };

    if let Some(mesh) = node.mesh() {
        let mut pick_positions = Vec::new();
        let mut pick_indices = Vec::new();
        let mut pick_min = Vec3::splat(f32::INFINITY);
        let mut pick_max = Vec3::splat(f32::NEG_INFINITY);

        for primitive in mesh.primitives() {
            if primitive.mode() != gltf::mesh::Mode::Triangles {
                continue;
            }
            let reader = primitive.reader(|buffer| buffers.get(buffer.index()).map(|v| &**v));
            let Some(pos_iter) = reader.read_positions() else {
                continue;
            };
            let raw_positions: Vec<[f32; 3]> = pos_iter.collect();
            if raw_positions.is_empty() {
                continue;
            }

            let raw_normals: Vec<[f32; 3]> = reader
                .read_normals()
                .map(|it| it.collect())
                .unwrap_or_else(|| vec![[0.0, 1.0, 0.0]; raw_positions.len()]);

            let mat = primitive.material();
            let base_factor = mat.pbr_metallic_roughness().base_color_factor();
            let emissive = mat.emissive_factor();

            let vertex_colors: Vec<[f32; 4]> = match reader.read_colors(0) {
                Some(ReadColors::RgbU8(it)) => it
                    .map(|c| {
                        [
                            (c[0] as f32 / 255.0) * base_factor[0],
                            (c[1] as f32 / 255.0) * base_factor[1],
                            (c[2] as f32 / 255.0) * base_factor[2],
                            base_factor[3],
                        ]
                    })
                    .collect(),
                Some(ReadColors::RgbU16(it)) => it
                    .map(|c| {
                        [
                            (c[0] as f32 / 65535.0) * base_factor[0],
                            (c[1] as f32 / 65535.0) * base_factor[1],
                            (c[2] as f32 / 65535.0) * base_factor[2],
                            base_factor[3],
                        ]
                    })
                    .collect(),
                Some(ReadColors::RgbF32(it)) => it
                    .map(|c| {
                        [
                            c[0] * base_factor[0],
                            c[1] * base_factor[1],
                            c[2] * base_factor[2],
                            base_factor[3],
                        ]
                    })
                    .collect(),
                Some(ReadColors::RgbaU8(it)) => it
                    .map(|c| {
                        [
                            (c[0] as f32 / 255.0) * base_factor[0],
                            (c[1] as f32 / 255.0) * base_factor[1],
                            (c[2] as f32 / 255.0) * base_factor[2],
                            (c[3] as f32 / 255.0) * base_factor[3],
                        ]
                    })
                    .collect(),
                Some(ReadColors::RgbaU16(it)) => it
                    .map(|c| {
                        [
                            (c[0] as f32 / 65535.0) * base_factor[0],
                            (c[1] as f32 / 65535.0) * base_factor[1],
                            (c[2] as f32 / 65535.0) * base_factor[2],
                            (c[3] as f32 / 65535.0) * base_factor[3],
                        ]
                    })
                    .collect(),
                Some(ReadColors::RgbaF32(it)) => it
                    .map(|c| {
                        [
                            c[0] * base_factor[0],
                            c[1] * base_factor[1],
                            c[2] * base_factor[2],
                            c[3] * base_factor[3],
                        ]
                    })
                    .collect(),
                None => {
                    let solid = [
                        (base_factor[0] + emissive[0] * 0.45).min(1.0),
                        (base_factor[1] + emissive[1] * 0.45).min(1.0),
                        (base_factor[2] + emissive[2] * 0.45).min(1.0),
                        base_factor[3],
                    ];
                    vec![solid; raw_positions.len()]
                }
            };

            let prim_indices: Vec<u32> = match reader.read_indices() {
                Some(ReadIndices::U8(it)) => it.map(|i| i as u32).collect(),
                Some(ReadIndices::U16(it)) => it.map(|i| i as u32).collect(),
                Some(ReadIndices::U32(it)) => it.collect(),
                None => (0..raw_positions.len() as u32).collect(),
            };

            let base_global_vertex = out_vertices.len() as u32;
            let base_pick_vertex = pick_positions.len() as u32;
            let is_province = matches!(kind, MeshKind::Province { .. });

            let pos_count = raw_positions.len() as u32;
            for i in 0..raw_positions.len() {
                let wp = world.transform_point3(Vec3::from(raw_positions[i]));
                let raw_n = raw_normals.get(i).copied().unwrap_or([0.0, 1.0, 0.0]);
                let wn = (normal_mat * Vec3::from(raw_n)).normalize_or_zero();
                let col = *vertex_colors.get(i).unwrap_or(&[0.5, 0.5, 0.5, 1.0]);

                *bounds_min = bounds_min.min(wp);
                *bounds_max = bounds_max.max(wp);

                out_vertices.push(Vertex3D {
                    position: wp.to_array(),
                    normal: wn.to_array(),
                    color: col,
                    meta: [country_slot as f32, province_slot as f32],
                });

                if is_province {
                    pick_min = pick_min.min(wp);
                    pick_max = pick_max.max(wp);
                    pick_positions.push(wp);
                }
            }

            for idx in prim_indices {
                if idx < pos_count {
                    out_indices.push(base_global_vertex + idx);
                    if is_province {
                        pick_indices.push(base_pick_vertex + idx);
                    }
                }
            }
        }

        if let MeshKind::Province {
            mesh_name,
            country_id,
        } = kind
        {
            if !pick_positions.is_empty() && !pick_indices.is_empty() {
                pick_meshes.push(ProvincePickMesh {
                    mesh_name,
                    country_id,
                    province_slot,
                    aabb_min: pick_min,
                    aabb_max: pick_max,
                    center: (pick_min + pick_max) * 0.5,
                    positions: pick_positions,
                    indices: pick_indices,
                });
            }
        }
    }

    for child in node.children() {
        visit_node(
            &child,
            world,
            buffers,
            out_vertices,
            out_indices,
            country_slots,
            province_slots,
            pick_meshes,
            bounds_min,
            bounds_max,
        );
    }
}

fn classify_node(name: &str) -> MeshKind {
    if name.starts_with("PROV_") {
        let parts: Vec<&str> = name.split('_').collect();
        let country_id = parts.get(1).copied().unwrap_or("XX").to_string();
        MeshKind::Province {
            mesh_name: name.to_string(),
            country_id,
        }
    } else if name.starts_with("CAP_") {
        let country_id = name
            .strip_prefix("CAP_")
            .unwrap_or("XX")
            .split('.')
            .next()
            .unwrap_or("XX")
            .to_string();
        MeshKind::Capital {
            node_name: name.to_string(),
            country_id,
            is_national: true,
        }
    } else if name.starts_with("PCAP_") {
        let parts: Vec<&str> = name.split('_').collect();
        let country_id = parts.get(1).copied().unwrap_or("XX").to_string();
        MeshKind::Capital {
            node_name: name.to_string(),
            country_id,
            is_national: false,
        }
    } else {
        MeshKind::Environment
    }
}

fn ray_intersects_aabb(
    origin: Vec3,
    dir: Vec3,
    min_b: Vec3,
    max_b: Vec3,
    max_dist: f32,
) -> bool {
    let mut tmin = 0.0_f32;
    let mut tmax = max_dist;
    for axis in 0..3 {
        let o = origin[axis];
        let d = dir[axis];
        let bmin = min_b[axis];
        let bmax = max_b[axis];
        if d.abs() < 1e-7 {
            if o < bmin || o > bmax {
                return false;
            }
        } else {
            let inv = 1.0 / d;
            let mut t1 = (bmin - o) * inv;
            let mut t2 = (bmax - o) * inv;
            if t1 > t2 {
                std::mem::swap(&mut t1, &mut t2);
            }
            tmin = tmin.max(t1);
            tmax = tmax.min(t2);
            if tmin > tmax {
                return false;
            }
        }
    }
    true
}

fn ray_triangle_intersect(
    origin: Vec3,
    dir: Vec3,
    v0: Vec3,
    v1: Vec3,
    v2: Vec3,
) -> Option<f32> {
    let eps = 1e-6;
    let e1 = v1 - v0;
    let e2 = v2 - v0;
    let pvec = dir.cross(e2);
    let det = e1.dot(pvec);
    if det.abs() < eps {
        return None;
    }
    let inv_det = 1.0 / det;
    let tvec = origin - v0;
    let u = tvec.dot(pvec) * inv_det;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let qvec = tvec.cross(e1);
    let v = dir.dot(qvec) * inv_det;
    if v < 0.0 || (u + v) > 1.0 {
        return None;
    }
    let t = e2.dot(qvec) * inv_det;
    if t > eps { Some(t) } else { None }
}
