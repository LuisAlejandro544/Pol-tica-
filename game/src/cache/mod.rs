//! Propósito del archivo:
//! Gestor modular de caché nativa en disco (`cacheDir` de Android o directorio temporal
//! en escritorio) comprimida con `postcard` + `lz4_flex` (licencias MIT / Apache-2.0).
//! Administra cuatro pilares:
//! 1. Pre-cocinado binario del Mapa 3D (`Latam3dScene`), catálogo provincial y `ProvinceGraph`.
//! 2. Caché de pipelines y shaders de `wgpu` por clave de GPU/driver Vulkan o GLES.
//! 3. Bytecode precompilado de Lua 5.4 (`provinces_latam`, `countries_latam`, `game_rules`).
//! 4. Buffer temporal en disco para Crónica antigua, puntos históricos bursátiles/macro y miniaturas.

use crate::map::glb_loader::{Latam3dScene, ProvincePickMesh, Vertex3D};
use crate::models::{
    ArchivedStockSample, CampaignThumbnail, CapitalMarkerData, ChronicleEntry,
    MacroHistorySnapshot, ProvinceData, ProvinceGraph,
};
use glam::Vec3;
use lz4_flex::block::{compress_prepend_size, decompress_size_prepended};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};

const MAP_CACHE_MAGIC: u32 = 0x484C_3344;
const MAP_CACHE_VERSION: u32 = 1;
const LUA_BYTECODE_CACHE_VERSION: u64 = 1;
const MAX_ARCHIVED_CHRONICLE: usize = 500;
const MAX_ARCHIVED_STOCK_SAMPLES: usize = 1_400;
const MAX_ARCHIVED_MACRO_SNAPSHOTS: usize = 840;

static CACHE_DIR: OnceLock<PathBuf> = OnceLock::new();
static WGPU_PIPELINE_KEY: OnceLock<String> = OnceLock::new();
static TMP_WRITE_SEQ: AtomicU64 = AtomicU64::new(1);

/// Inicializa el directorio raíz de caché nativa (en Android apunta a `<app>/cache`).
pub fn init_cache_dir(preferred: Option<PathBuf>) {
    let resolved = preferred.unwrap_or_else(default_fallback_cache_dir);
    let _ = fs::create_dir_all(&resolved);
    let _ = CACHE_DIR.set(resolved);
}

fn default_fallback_cache_dir() -> PathBuf {
    std::env::temp_dir().join("horizonte_latam_cache")
}

/// Devuelve el directorio raíz de caché garantizando que exista en disco.
pub fn cache_dir() -> PathBuf {
    let path = CACHE_DIR
        .get_or_init(|| {
            let p = default_fallback_cache_dir();
            let _ = fs::create_dir_all(&p);
            p
        })
        .clone();
    let _ = fs::create_dir_all(&path);
    path
}

/// Registra la clave de adaptador/driver GPU usada para la caché de pipelines `wgpu`.
pub fn set_wgpu_pipeline_cache_key(key: String) {
    let sanitized = sanitize_key(&key);
    if !sanitized.is_empty() {
        let _ = WGPU_PIPELINE_KEY.set(sanitized);
    }
}

pub fn wgpu_pipeline_cache_key() -> Option<&'static str> {
    WGPU_PIPELINE_KEY.get().map(String::as_str)
}

fn sanitize_key(raw: &str) -> String {
    raw.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Hash FNV-1a determinista de 64 bits (idéntico en Android 32-bit y 64-bit).
pub fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xCBF2_9CE4_8422_2325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
    }
    hash
}

fn hash_step(acc: u64, data: &[u8]) -> u64 {
    let len_bytes = (data.len() as u64).to_le_bytes();
    let mut h = acc ^ fnv1a_64(data);
    h = h.rotate_left(13) ^ fnv1a_64(&len_bytes);
    h.wrapping_mul(0x9E37_79B9_7F4A_7C15)
}

/// Huella combinada de `latinoamerica_3d.glb` y `provinces_latam.lua` para invalidar el mapa pre-cocinado.
pub fn compute_scene_fingerprint(glb_bytes: &[u8], provinces_lua_bytes: &[u8]) -> u64 {
    let mut h = (MAP_CACHE_MAGIC as u64) << 32 | (MAP_CACHE_VERSION as u64);
    h = hash_step(h, glb_bytes);
    h = hash_step(h, provinces_lua_bytes);
    h
}

/// Huella de un script Lua 5.4 que incluye ancho de puntero (`usize::BITS`) por compatibilidad de bytecode.
pub fn compute_lua_fingerprint(script_name: &str, src: &str) -> u64 {
    let mut h = LUA_BYTECODE_CACHE_VERSION ^ (usize::BITS as u64).wrapping_mul(0x517C_C1B7_2722_0A95);
    h = hash_step(h, script_name.as_bytes());
    h = hash_step(h, src.as_bytes());
    h
}

fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let seq = TMP_WRITE_SEQ.fetch_add(1, Ordering::Relaxed);
    let file_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("cache.bin");
    let tmp_path = path.with_file_name(format!(".{file_name}.{seq}.tmp"));
    fs::write(&tmp_path, bytes)?;
    if let Err(err) = fs::rename(&tmp_path, path) {
        let _ = fs::remove_file(&tmp_path);
        return Err(err);
    }
    Ok(())
}

// ============================================================================
// 1. Pre-cocinado binario del Mapa 3D, Catálogo Provincial y ProvinceGraph
// ============================================================================

#[derive(Serialize, Deserialize)]
struct BakedPickMesh {
    mesh_name: String,
    country_id: String,
    province_slot: i32,
    aabb_min: [f32; 3],
    aabb_max: [f32; 3],
    center: [f32; 3],
    positions: Vec<[f32; 3]>,
    indices: Vec<u32>,
}

#[derive(Serialize, Deserialize)]
struct BakedMapBundle {
    magic: u32,
    version: u32,
    fingerprint: u64,
    vertices: Vec<Vertex3D>,
    indices: Vec<u32>,
    country_slots: BTreeMap<String, i32>,
    province_slots: BTreeMap<String, i32>,
    pick_meshes: Vec<BakedPickMesh>,
    bounds_min: [f32; 3],
    bounds_max: [f32; 3],
    default_center: [f32; 3],
    provinces_catalog: BTreeMap<String, ProvinceData>,
    capitals_catalog: Vec<CapitalMarkerData>,
    province_graph: ProvinceGraph,
}

fn baked_map_path() -> PathBuf {
    cache_dir().join("map").join("map_scene_v1.hlcache")
}

/// Intenta cargar desde caché binaria comprimida el mapa 3D pre-cocinado y su `ProvinceGraph`.
pub fn load_baked_map_bundle(
    expected_fingerprint: u64,
) -> Option<(
    Latam3dScene,
    BTreeMap<String, ProvinceData>,
    Vec<CapitalMarkerData>,
    ProvinceGraph,
)> {
    let raw = fs::read(baked_map_path()).ok()?;
    let decompressed = decompress_size_prepended(&raw).ok()?;
    let bundle: BakedMapBundle = postcard::from_bytes(&decompressed).ok()?;

    if bundle.magic != MAP_CACHE_MAGIC
        || bundle.version != MAP_CACHE_VERSION
        || bundle.fingerprint != expected_fingerprint
    {
        return None;
    }

    let pick_meshes = bundle
        .pick_meshes
        .into_iter()
        .map(|pm| ProvincePickMesh {
            mesh_name: pm.mesh_name,
            country_id: pm.country_id,
            province_slot: pm.province_slot,
            aabb_min: Vec3::from(pm.aabb_min),
            aabb_max: Vec3::from(pm.aabb_max),
            center: Vec3::from(pm.center),
            positions: pm.positions.into_iter().map(Vec3::from).collect(),
            indices: pm.indices,
        })
        .collect();

    let scene = Latam3dScene {
        vertices: Arc::new(bundle.vertices),
        indices: Arc::new(bundle.indices),
        country_slots: bundle.country_slots,
        province_slots: bundle.province_slots,
        pick_meshes,
        bounds_min: Vec3::from(bundle.bounds_min),
        bounds_max: Vec3::from(bundle.bounds_max),
        default_center: Vec3::from(bundle.default_center),
    };

    Some((
        scene,
        bundle.provinces_catalog,
        bundle.capitals_catalog,
        bundle.province_graph,
    ))
}

/// Serializa y comprime el mapa 3D, catálogo y grafo provincial en caché binaria.
pub fn save_baked_map_bundle(
    fingerprint: u64,
    scene: &Latam3dScene,
    provinces_catalog: &BTreeMap<String, ProvinceData>,
    capitals_catalog: &[CapitalMarkerData],
    province_graph: &ProvinceGraph,
) {
    let pick_meshes = scene
        .pick_meshes
        .iter()
        .map(|pm| BakedPickMesh {
            mesh_name: pm.mesh_name.clone(),
            country_id: pm.country_id.clone(),
            province_slot: pm.province_slot,
            aabb_min: pm.aabb_min.to_array(),
            aabb_max: pm.aabb_max.to_array(),
            center: pm.center.to_array(),
            positions: pm.positions.iter().map(|v| v.to_array()).collect(),
            indices: pm.indices.clone(),
        })
        .collect();

    let bundle = BakedMapBundle {
        magic: MAP_CACHE_MAGIC,
        version: MAP_CACHE_VERSION,
        fingerprint,
        vertices: (*scene.vertices).clone(),
        indices: (*scene.indices).clone(),
        country_slots: scene.country_slots.clone(),
        province_slots: scene.province_slots.clone(),
        pick_meshes,
        bounds_min: scene.bounds_min.to_array(),
        bounds_max: scene.bounds_max.to_array(),
        default_center: scene.default_center.to_array(),
        provinces_catalog: provinces_catalog.clone(),
        capitals_catalog: capitals_catalog.to_vec(),
        province_graph: province_graph.clone(),
    };

    let Ok(encoded) = postcard::to_allocvec(&bundle) else {
        return;
    };
    let compressed = compress_prepend_size(&encoded);
    if let Err(err) = write_atomic(&baked_map_path(), &compressed) {
        log::warn!("No se pudo guardar caché binaria del mapa 3D: {err}");
    }
}

// ============================================================================
// 2. Caché de Pipeline y Shaders de wgpu
// ============================================================================

fn wgpu_pipeline_path(key: &str) -> PathBuf {
    let clean = sanitize_key(key);
    cache_dir().join("shaders").join(format!("{clean}.bin"))
}

pub fn load_wgpu_pipeline_cache(key: &str) -> Option<Vec<u8>> {
    if key.is_empty() {
        return None;
    }
    fs::read(wgpu_pipeline_path(key)).ok()
}

pub fn save_wgpu_pipeline_cache(key: &str, bytes: &[u8]) {
    if key.is_empty() || bytes.is_empty() {
        return;
    }
    if let Err(err) = write_atomic(&wgpu_pipeline_path(key), bytes) {
        log::warn!("No se pudo guardar caché de shaders wgpu: {err}");
    }
}

// ============================================================================
// 3. Caché de Bytecode precompilado de Lua 5.4
// ============================================================================

fn lua_bytecode_dir() -> PathBuf {
    cache_dir().join("lua")
}

fn lua_bytecode_path(script_name: &str, fingerprint: u64) -> PathBuf {
    let clean = sanitize_key(script_name);
    lua_bytecode_dir().join(format!("{clean}_{fingerprint:016x}.luac"))
}

pub fn load_lua_bytecode(script_name: &str, fingerprint: u64) -> Option<Vec<u8>> {
    fs::read(lua_bytecode_path(script_name, fingerprint)).ok()
}

pub fn save_lua_bytecode(script_name: &str, fingerprint: u64, bytecode: &[u8]) {
    if bytecode.is_empty() {
        return;
    }
    let dir = lua_bytecode_dir();
    let clean = sanitize_key(script_name);
    let prefix = format!("{clean}_");
    let target = lua_bytecode_path(script_name, fingerprint);

    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p != target {
                if let Some(name) = p.file_name().and_then(|s| s.to_str()) {
                    if name.starts_with(&prefix) && name.ends_with(".luac") {
                        let _ = fs::remove_file(p);
                    }
                }
            }
        }
    }

    if let Err(err) = write_atomic(&target, bytecode) {
        log::warn!("No se pudo guardar bytecode Lua {script_name}: {err}");
    }
}

// ============================================================================
// 4. Buffer temporal de Crónica antigua, Gráficas históricas y Miniaturas
// ============================================================================

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CampaignHistoryArchive {
    pub campaign_seed: u64,
    pub archived_chronicle: Vec<ChronicleEntry>,
    pub archived_stock_samples: Vec<ArchivedStockSample>,
    pub macro_snapshots: Vec<MacroHistorySnapshot>,
}

fn history_archive_path(campaign_seed: u64) -> PathBuf {
    cache_dir()
        .join("history")
        .join(format!("campaign_{campaign_seed:016x}.hlcache"))
}

pub fn load_campaign_history_archive(campaign_seed: u64) -> Option<CampaignHistoryArchive> {
    let raw = fs::read(history_archive_path(campaign_seed)).ok()?;
    let decompressed = decompress_size_prepended(&raw).ok()?;
    postcard::from_bytes(&decompressed).ok()
}

/// Vuelca entradas antiguas de Crónica, muestras viejas de acciones e instantáneas macro a disco comprimido.
pub fn append_campaign_history_batch(
    campaign_seed: u64,
    chronicle_batch: &[ChronicleEntry],
    stock_batch: &[ArchivedStockSample],
    macro_batch: &[MacroHistorySnapshot],
) {
    if chronicle_batch.is_empty() && stock_batch.is_empty() && macro_batch.is_empty() {
        return;
    }

    let mut archive = load_campaign_history_archive(campaign_seed).unwrap_or_else(|| {
        CampaignHistoryArchive {
            campaign_seed,
            ..Default::default()
        }
    });

    if !chronicle_batch.is_empty() {
        archive
            .archived_chronicle
            .extend(chronicle_batch.iter().cloned());
        if archive.archived_chronicle.len() > MAX_ARCHIVED_CHRONICLE {
            let excess = archive.archived_chronicle.len() - MAX_ARCHIVED_CHRONICLE;
            archive.archived_chronicle.drain(0..excess);
        }
    }

    if !stock_batch.is_empty() {
        archive
            .archived_stock_samples
            .extend(stock_batch.iter().cloned());
        if archive.archived_stock_samples.len() > MAX_ARCHIVED_STOCK_SAMPLES {
            let excess = archive.archived_stock_samples.len() - MAX_ARCHIVED_STOCK_SAMPLES;
            archive.archived_stock_samples.drain(0..excess);
        }
    }

    if !macro_batch.is_empty() {
        archive.macro_snapshots.extend(macro_batch.iter().cloned());
        if archive.macro_snapshots.len() > MAX_ARCHIVED_MACRO_SNAPSHOTS {
            let excess = archive.macro_snapshots.len() - MAX_ARCHIVED_MACRO_SNAPSHOTS;
            archive.macro_snapshots.drain(0..excess);
        }
    }

    if let Ok(encoded) = postcard::to_allocvec(&archive) {
        let compressed = compress_prepend_size(&encoded);
        let _ = write_atomic(&history_archive_path(campaign_seed), &compressed);
    }
}

fn thumbnail_path(country_id: &str) -> PathBuf {
    let clean = sanitize_key(country_id);
    cache_dir()
        .join("thumbnails")
        .join(format!("thumb_{clean}.hlcache"))
}

pub fn save_campaign_thumbnail(thumb: &CampaignThumbnail) {
    if let Ok(encoded) = postcard::to_allocvec(thumb) {
        let compressed = compress_prepend_size(&encoded);
        let _ = write_atomic(&thumbnail_path(&thumb.country_id), &compressed);
    }
}

pub fn load_campaign_thumbnails() -> Vec<CampaignThumbnail> {
    let dir = cache_dir().join("thumbnails");
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.extension().and_then(|s| s.to_str()) != Some("hlcache") {
            continue;
        }
        if let Ok(raw) = fs::read(p) {
            if let Ok(dec) = decompress_size_prepended(&raw) {
                if let Ok(thumb) = postcard::from_bytes::<CampaignThumbnail>(&dec) {
                    out.push(thumb);
                }
            }
        }
    }
    out
}

#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub map_bytes: u64,
    pub shader_bytes: u64,
    pub lua_bytes: u64,
    pub history_bytes: u64,
    pub total_bytes: u64,
}

fn dir_size_bytes(path: &Path) -> u64 {
    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };
    let mut total = 0_u64;
    for entry in entries.flatten() {
        if let Ok(meta) = entry.metadata() {
            if meta.is_file() {
                total = total.saturating_add(meta.len());
            } else if meta.is_dir() {
                total = total.saturating_add(dir_size_bytes(&entry.path()));
            }
        }
    }
    total
}

/// Inspecciona el espacio usado por cada bloque de la caché nativa.
pub fn inspect_cache_stats() -> CacheStats {
    let root = cache_dir();
    let map_bytes = dir_size_bytes(&root.join("map"));
    let shader_bytes = dir_size_bytes(&root.join("shaders"));
    let lua_bytes = dir_size_bytes(&root.join("lua"));
    let history_bytes =
        dir_size_bytes(&root.join("history")).saturating_add(dir_size_bytes(&root.join("thumbnails")));
    let total_bytes = map_bytes
        .saturating_add(shader_bytes)
        .saturating_add(lua_bytes)
        .saturating_add(history_bytes);
    CacheStats {
        map_bytes,
        shader_bytes,
        lua_bytes,
        history_bytes,
        total_bytes,
    }
}

/// Borra los archivos de caché nativa y recrea las carpetas vacías.
pub fn clear_all_cache() -> std::io::Result<()> {
    let root = cache_dir();
    for sub in ["map", "shaders", "lua", "history", "thumbnails"] {
        let p = root.join(sub);
        if p.exists() {
            fs::remove_dir_all(&p)?;
        }
        fs::create_dir_all(&p)?;
    }
    Ok(())
}
