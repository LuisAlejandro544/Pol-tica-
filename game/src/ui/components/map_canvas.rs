//! Propósito del archivo:
//! Componente interactivo reutilizable del visor cartográfico 3D de Latinoamérica.
//! Renderiza exclusivamente en 3D real (`wgpu 25` + `latinoamerica_3d.glb`) con pan táctil,
//! zoom por pellizco/rueda, rotación/inclinación 3D, modos dinámicos de mapa y raycasting
//! exacto por provincia y país. Sin respaldo 2D SVG.

use crate::map::renderer_3d::{srgb_u8_to_linear, Map3dCamera, Map3dPaintCallback, MapUniforms};
use crate::map::Latam3dScene;
use crate::models::{GameState, MapMode};
use crate::ui::theme::{
    ACCENT_CRIMSON, ACCENT_CYAN, ACCENT_EMERALD, ACCENT_GOLD, BORDER_SUBTLE, TEXT_MUTED,
    TEXT_PRIMARY,
};
use eframe::egui_wgpu::{self, wgpu};
use egui::{Color32, Rect, RichText, Sense, Stroke, Ui, Vec2};
use parking_lot::Mutex;
use std::sync::Arc;

pub struct MapPickHit {
    pub country_id: String,
    pub province_mesh: Option<String>,
}

pub struct MapViewportState {
    pub zoom: f32,
    pub pan: Vec2,
    pub camera_3d: Map3dCamera,
}

impl Default for MapViewportState {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: Vec2::ZERO,
            camera_3d: Map3dCamera::default(),
        }
    }
}

/// Dibuja el visor 3D (`wgpu`) y devuelve el país y provincia tocados si hubo tap/clic.
pub fn render_latam_map(
    ui: &mut Ui,
    scene_3d: Option<&Latam3dScene>,
    target_format: wgpu::TextureFormat,
    gpu_info: &str,
    gpu_3d_error: &Arc<Mutex<Option<String>>>,
    state: &GameState,
    map_mode: MapMode,
    viewport: &mut MapViewportState,
) -> Option<MapPickHit> {
    let available = ui.available_size();
    let safe_size = Vec2::new(
        if available.x.is_finite() { available.x.max(16.0) } else { 320.0 },
        if available.y.is_finite() { available.y.max(16.0) } else { 240.0 },
    );
    let (rect, response) = ui.allocate_exact_size(safe_size, Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 8.0, Color32::from_rgb(8, 22, 38));

    let Some(scene) = scene_3d else {
        painter.rect_stroke(
            rect,
            8.0,
            Stroke::new(1.5_f32, BORDER_SUBTLE),
            egui::StrokeKind::Inside,
        );
        let msg_rect = rect.shrink(16.0);
        let mut msg_ui = ui.new_child(egui::UiBuilder::new().max_rect(msg_rect));
        msg_ui.vertical(|ui| {
            ui.label(
                RichText::new("No se encontró el modelo 3D latinoamerica_3d.glb")
                    .strong()
                    .color(ACCENT_CRIMSON),
            );
            ui.label(
                RichText::new(format!("GPU: {gpu_info} • Formato: {target_format:?}"))
                    .size(12.0)
                    .color(TEXT_MUTED),
            );
        });
        return None;
    };

    if let Some(err) = gpu_3d_error.lock().clone() {
        painter.rect_stroke(
            rect,
            8.0,
            Stroke::new(1.5_f32, ACCENT_CRIMSON),
            egui::StrokeKind::Inside,
        );
        let msg_rect = rect.shrink(16.0);
        let mut msg_ui = ui.new_child(egui::UiBuilder::new().max_rect(msg_rect));
        msg_ui.vertical(|ui| {
            ui.label(
                RichText::new("Error en visor 3D (wgpu)")
                    .strong()
                    .color(ACCENT_CRIMSON),
            );
            ui.label(
                RichText::new(format!("GPU: {gpu_info} • Formato: {target_format:?}"))
                    .size(12.0)
                    .color(TEXT_MUTED),
            );
            ui.add_space(6.0);
            ui.label(RichText::new(err).size(12.5).color(TEXT_PRIMARY));
            ui.add_space(10.0);
            if ui.button("Reintentar Visor 3D").clicked() {
                *gpu_3d_error.lock() = None;
            }
        });
        return None;
    }

    // Gestos táctiles y de ratón sobre el visor 3D
    if response.hovered() {
        if let Some(mt) = ui.input(|i| i.multi_touch()) {
            if (mt.zoom_delta - 1.0).abs() > 0.002 {
                viewport.camera_3d.zoom_by_factor(mt.zoom_delta);
            }
            if mt.translation_delta.length_sq() > 0.5 {
                viewport.camera_3d.orbit_by_delta(mt.translation_delta);
            }
        } else if response.dragged_by(egui::PointerButton::Secondary) {
            viewport.camera_3d.orbit_by_delta(response.drag_delta());
        } else if response.dragged_by(egui::PointerButton::Primary) {
            viewport
                .camera_3d
                .pan_by_screen_delta(response.drag_delta(), rect.height());
        }

        let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll_delta.abs() > 0.1 {
            let factor = (1.0 + scroll_delta * 0.0025).clamp(0.8, 1.25);
            viewport.camera_3d.zoom_by_factor(factor);
        }
    }

    // Construcción de paleta cromática por país según capa seleccionada
    let mut country_palette = [[0.5_f32, 0.5, 0.5, 1.0]; 32];
    for (cid, &slot) in &scene.country_slots {
        if (0..32).contains(&slot) {
            let is_player = state.player_country_id.as_deref() == Some(cid.as_str());
            let fill = compute_country_fill(cid, state, map_mode, false, is_player);
            let lin = srgb_u8_to_linear([fill.r(), fill.g(), fill.b()]);
            country_palette[slot as usize] = [lin[0], lin[1], lin[2], 1.0];
        }
    }

    let sel_country_slot = state
        .selected_country_id
        .as_deref()
        .and_then(|id| scene.country_slots.get(id).copied())
        .unwrap_or(-1);
    let player_country_slot = state
        .player_country_id
        .as_deref()
        .and_then(|id| scene.country_slots.get(id).copied())
        .unwrap_or(-1);
    let sel_prov_slot = state
        .selected_province_mesh
        .as_deref()
        .and_then(|m| scene.province_slots.get(m).copied())
        .unwrap_or(-1);
    let mode_id = match map_mode {
        MapMode::Political => 0,
        MapMode::GdpPerCapita => 1,
        MapMode::Inflation => 2,
        MapMode::Diplomacy => 3,
    };

    let aspect = (rect.width() / rect.height().max(1.0)).max(0.1);
    let view_proj = viewport.camera_3d.view_proj(aspect);
    let eye = viewport.camera_3d.eye_position();
    let ppp = ui.ctx().pixels_per_point().clamp(0.5, 4.0);
    let vp_w = ((rect.width() * ppp).round() as u32).clamp(64, 2048);
    let vp_h = ((rect.height() * ppp).round() as u32).clamp(64, 2048);

    let uniforms = MapUniforms {
        view_proj: view_proj.to_cols_array_2d(),
        camera_pos: [eye.x, eye.y, eye.z, 1.0],
        flags: [sel_country_slot, player_country_slot, sel_prov_slot, mode_id],
        country_palette,
    };

    if rect.is_positive() && rect.width() >= 16.0 && rect.height() >= 16.0 {
        let callback = egui_wgpu::Callback::new_paint_callback(
            rect,
            Map3dPaintCallback {
                vertices: scene.vertices.clone(),
                indices: scene.indices.clone(),
                uniforms,
                target_format,
                viewport_px: [vp_w, vp_h],
                gpu_3d_error: Arc::clone(gpu_3d_error),
            },
        );
        painter.add(callback);
    }

    // Marco sutil cartográfico alrededor del visor 3D
    painter.rect_stroke(
        rect,
        8.0,
        Stroke::new(1.5_f32, BORDER_SUBTLE),
        egui::StrokeKind::Inside,
    );

    // Controles táctiles flotantes en la esquina inferior derecha del visor 3D
    let bar_w = (rect.width() - 16.0).clamp(150.0, 278.0);
    let btn_rect = Rect::from_min_size(
        rect.right_bottom() - Vec2::new(bar_w + 8.0, 44.0),
        Vec2::new(bar_w, 34.0),
    );
    let mut ui_child = ui.new_child(egui::UiBuilder::new().max_rect(btn_rect));
    ui_child.horizontal_wrapped(|ui| {
        if ui.small_button(" + ").clicked() {
            viewport.camera_3d.zoom_by_factor(1.24);
        }
        if ui.small_button(" - ").clicked() {
            viewport.camera_3d.zoom_by_factor(1.0 / 1.24);
        }
        if ui.small_button(" < ").clicked() {
            viewport.camera_3d.yaw = (viewport.camera_3d.yaw - 0.18).clamp(-1.35, 1.35);
        }
        if ui.small_button(" > ").clicked() {
            viewport.camera_3d.yaw = (viewport.camera_3d.yaw + 0.18).clamp(-1.35, 1.35);
        }
        let angle_lbl = if viewport.camera_3d.is_top_down() {
            "3D"
        } else {
            "Cenital"
        };
        if ui.small_button(angle_lbl).clicked() {
            if viewport.camera_3d.is_top_down() {
                viewport.camera_3d.set_tactical_3d();
            } else {
                viewport.camera_3d.set_top_down();
            }
        }
        if ui.small_button("Centrar").clicked() {
            viewport.camera_3d.reset();
        }
    });

    // Raycasting 3D exacto al tocar/clicar sobre una provincia
    if response.clicked() {
        if let Some(pointer_pos) = response.interact_pointer_pos() {
            let (ray_origin, ray_dir) = viewport.camera_3d.screen_to_ray(pointer_pos, rect);
            if let Some((mesh_name, country_id)) = scene.pick_province_at_ray(ray_origin, ray_dir) {
                return Some(MapPickHit {
                    country_id: country_id.to_string(),
                    province_mesh: Some(mesh_name.to_string()),
                });
            }
        }
    }

    None
}

fn compute_country_fill(
    id: &str,
    state: &GameState,
    mode: MapMode,
    is_selected: bool,
    is_player: bool,
) -> Color32 {
    let Some(country) = state.countries.get(id) else {
        return Color32::DARK_GRAY;
    };

    let mut base = match mode {
        MapMode::Political => Color32::from_rgb(country.color[0], country.color[1], country.color[2]),
        MapMode::GdpPerCapita => {
            let pc = country.gdp_per_capita_usd();
            let t = ((pc - 2_000.0) / 18_000.0).clamp(0.0, 1.0) as f32;
            lerp_color(Color32::from_rgb(132, 54, 54), ACCENT_EMERALD, t)
        }
        MapMode::Inflation => {
            let t = ((country.economy.inflation_pct - 2.0) / 35.0).clamp(0.0, 1.0) as f32;
            lerp_color(ACCENT_EMERALD, ACCENT_CRIMSON, t)
        }
        MapMode::Diplomacy => {
            if is_player {
                ACCENT_CYAN
            } else if let Some(pid) = &state.player_country_id {
                let rel = state.get_relation(pid, id);
                if rel.at_war {
                    ACCENT_CRIMSON
                } else if rel.alliance {
                    ACCENT_CYAN
                } else if rel.trade_pact {
                    ACCENT_EMERALD
                } else {
                    let t = ((rel.opinion + 100.0) / 200.0).clamp(0.0, 1.0) as f32;
                    lerp_color(
                        Color32::from_rgb(160, 64, 64),
                        Color32::from_rgb(64, 155, 108),
                        t,
                    )
                }
            } else {
                Color32::from_rgb(country.color[0], country.color[1], country.color[2])
            }
        }
    };

    if is_selected {
        base = lerp_color(base, ACCENT_GOLD, 0.32);
    }
    base
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let r = (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8;
    let g = (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8;
    let bl = (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8;
    Color32::from_rgb(r, g, bl)
}
