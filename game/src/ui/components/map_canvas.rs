//! Propósito del archivo:
//! Componente interactivo reutilizable del visor cartográfico 3D de Latinoamérica.
//! Renderiza exclusivamente en 3D real (`wgpu 25` + `latinoamerica_3d.glb`) con pan táctil,
//! zoom por pellizco/rueda, rotación/inclinación 3D, modos dinámicos de mapa, marco táctico
//! con controles de cámara en botones físicos y raycasting exacto por provincia y país.

use crate::map::renderer_3d::{srgb_u8_to_linear, Map3dCamera, Map3dPaintCallback, MapUniforms};
use crate::map::Latam3dScene;
use crate::models::{GameState, MapMode};
use crate::ui::theme::{
    draw_corner_rivets, floating_hud_frame, step_button, GameIcon, ACCENT_CRIMSON, ACCENT_CYAN,
    ACCENT_EMERALD, ACCENT_GOLD, BG_PANEL, BG_WELL, BORDER_BRASS, BORDER_SUBTLE, TEXT_DARK,
    TEXT_MUTED, TEXT_PRIMARY,
};
use eframe::egui_wgpu::{self, wgpu};
use egui::{Color32, CornerRadius, FontId, Margin, Pos2, Rect, RichText, Sense, Stroke, Ui, Vec2};
use glam::{Vec3, Vec4};
use parking_lot::Mutex;
use std::sync::Arc;

pub struct MapPickHit {
    pub country_id: String,
    pub province_mesh: Option<String>,
    pub unit_id: Option<u64>,
}

#[derive(Debug, Clone, Copy)]
pub struct MapCanvasConfig {
    pub fullscreen_borderless: bool,
    pub show_mode_badge: bool,
    pub camera_bottom_offset: f32,
    pub camera_right_inset: f32,
}

impl Default for MapCanvasConfig {
    fn default() -> Self {
        Self {
            fullscreen_borderless: false,
            show_mode_badge: true,
            camera_bottom_offset: 8.0,
            camera_right_inset: 10.0,
        }
    }
}

pub struct MapViewportState {
    pub zoom: f32,
    pub pan: Vec2,
    pub camera_3d: Map3dCamera,
    pub show_chronicle_drawer: bool,
    pub hud_rects: Vec<Rect>,
}

impl Default for MapViewportState {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: Vec2::ZERO,
            camera_3d: Map3dCamera::default(),
            show_chronicle_drawer: false,
            hud_rects: Vec::new(),
        }
    }
}

/// Dibuja el visor 3D (`wgpu`) y devuelve el país, provincia o unidad tocados si hubo tap/clic.
#[allow(clippy::too_many_arguments)]
pub fn render_latam_map(
    ui: &mut Ui,
    scene_3d: Option<&Latam3dScene>,
    target_format: wgpu::TextureFormat,
    gpu_info: &str,
    gpu_3d_error: &Arc<Mutex<Option<String>>>,
    state: &GameState,
    map_mode: MapMode,
    viewport: &mut MapViewportState,
    config: MapCanvasConfig,
) -> Option<MapPickHit> {
    let available = ui.available_size();
    let safe_size = Vec2::new(
        if available.x.is_finite() { available.x.max(16.0) } else { 320.0 },
        if available.y.is_finite() { available.y.max(16.0) } else { 240.0 },
    );
    let (rect, response) = ui.allocate_exact_size(safe_size, Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, CornerRadius::same(4), Color32::from_rgb(8, 20, 34));

    let Some(scene) = scene_3d else {
        painter.rect_stroke(
            rect,
            CornerRadius::same(4),
            Stroke::new(1.5_f32, BORDER_BRASS),
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
            CornerRadius::same(4),
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
            if step_button(ui, Vec2::new(170.0, 36.0), "Reintentar Visor 3D", Some(GameIcon::Center))
                .clicked()
            {
                *gpu_3d_error.lock() = None;
            }
        });
        return None;
    }

    let pointer_latest = ui.ctx().input(|i| i.pointer.latest_pos());
    let pointer_on_hud = pointer_latest
        .map(|p| viewport.hud_rects.iter().any(|r| r.contains(p)))
        .unwrap_or(false);

    // Gestos táctiles y de ratón sobre el visor 3D (ignorados si el dedo está sobre un HUD flotante)
    if response.hovered() && !pointer_on_hud {
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

    if !config.fullscreen_borderless {
        // Marco cartográfico de latón y acero con remaches en las esquinas para vistas en ventana
        painter.rect_stroke(
            rect,
            CornerRadius::same(4),
            Stroke::new(1.5_f32, BORDER_BRASS),
            egui::StrokeKind::Inside,
        );
        draw_corner_rivets(&painter, rect, BORDER_BRASS);
    }

    // Proyección 3D -> 2D de unidades militares desplegadas con dispersión anti-solapamiento
    let mut unit_hitboxes: Vec<(Rect, u64, String)> = Vec::new();
    let mut placed_badges: Vec<Rect> = Vec::new();
    if state.player_country_id.is_some() {
        let map_painter = painter.with_clip_rect(rect.shrink(2.0));
        for unit in &state.units {
            let Some(anchor_pos) = project_world_to_rect(unit.world_pos, view_proj, rect) else {
                continue;
            };
            let is_player_unit =
                state.player_country_id.as_deref() == Some(unit.country_id.as_str());
            let is_selected = state.selected_unit_id == Some(unit.id);

            // Línea de ruta hacia la provincia objetivo si está marchando
            if let Some(target_mesh) = &unit.target_province {
                if let Some(dest_mesh) = scene.pick_meshes.iter().find(|m| m.mesh_name == *target_mesh) {
                    let dest_world = [
                        dest_mesh.center.x,
                        dest_mesh.center.y + 0.35,
                        dest_mesh.center.z,
                    ];
                    if let Some(dest_screen) = project_world_to_rect(dest_world, view_proj, rect) {
                        let line_col = if is_selected {
                            ACCENT_GOLD
                        } else if is_player_unit {
                            ACCENT_CYAN
                        } else {
                            ACCENT_CRIMSON
                        };
                        map_painter.line_segment(
                            [anchor_pos, dest_screen],
                            Stroke::new(2.2_f32, line_col),
                        );
                        map_painter.circle_filled(dest_screen, 4.0, line_col);
                    }
                }
            }

            let badge_size = if is_selected {
                Vec2::new(46.0, 18.0)
            } else {
                Vec2::new(37.0, 15.5)
            };

            let offsets = [
                Vec2::ZERO,
                Vec2::new(0.0, -19.0),
                Vec2::new(0.0, 19.0),
                Vec2::new(41.0, 0.0),
                Vec2::new(-41.0, 0.0),
                Vec2::new(41.0, -19.0),
                Vec2::new(-41.0, 19.0),
                Vec2::new(-41.0, -19.0),
                Vec2::new(41.0, 19.0),
                Vec2::new(0.0, -38.0),
                Vec2::new(0.0, 38.0),
                Vec2::new(82.0, 0.0),
                Vec2::new(-82.0, 0.0),
                Vec2::new(41.0, -38.0),
                Vec2::new(-41.0, -38.0),
                Vec2::new(41.0, 38.0),
                Vec2::new(-41.0, 38.0),
            ];
            let mut badge_rect = Rect::from_center_size(anchor_pos, badge_size);
            let mut found_slot = false;
            for off in offsets {
                let candidate = Rect::from_center_size(anchor_pos + off, badge_size);
                if !placed_badges.iter().any(|r| r.expand(2.5).intersects(candidate)) {
                    badge_rect = candidate;
                    found_slot = true;
                    break;
                }
            }
            if !found_slot {
                let ring = (placed_badges.len() as f32) * 0.85;
                badge_rect = Rect::from_center_size(
                    anchor_pos + Vec2::new(ring.cos() * 44.0, ring.sin() * 24.0),
                    badge_size,
                );
            }
            placed_badges.push(badge_rect);

            if (badge_rect.center() - anchor_pos).length_sq() > 9.0 {
                map_painter.line_segment(
                    [anchor_pos, badge_rect.center()],
                    Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(194, 152, 74, 150)),
                );
                map_painter.circle_filled(anchor_pos, 2.2, ACCENT_GOLD);
            }

            let c_rgb = state
                .countries
                .get(&unit.country_id)
                .map(|c| Color32::from_rgb(c.color[0], c.color[1], c.color[2]))
                .unwrap_or(Color32::GRAY);

            let border_col = if is_selected {
                ACCENT_GOLD
            } else if is_player_unit {
                ACCENT_CYAN
            } else {
                Color32::from_rgb(70, 88, 112)
            };
            let fill_col = if is_selected {
                ACCENT_GOLD
            } else {
                Color32::from_rgba_unmultiplied(12, 18, 28, 232)
            };

            // Sombra sutil debajo de la ficha táctica
            map_painter.rect_filled(
                badge_rect.translate(Vec2::new(1.5, 2.0)),
                CornerRadius::same(3),
                Color32::from_rgba_unmultiplied(0, 0, 0, 125),
            );
            map_painter.rect_filled(badge_rect, CornerRadius::same(3), fill_col);
            map_painter.rect_stroke(
                badge_rect,
                CornerRadius::same(3),
                Stroke::new(if is_selected { 2.0_f32 } else { 1.2_f32 }, border_col),
                egui::StrokeKind::Inside,
            );

            // Franja lateral de color nacional
            let stripe = Rect::from_min_size(
                badge_rect.min + Vec2::new(1.5, 1.5),
                Vec2::new(4.0, badge_rect.height() - 3.0),
            );
            map_painter.rect_filled(stripe, CornerRadius::same(1), c_rgb);

            let troops_k = (unit.troops as f64) / 1_000.0;
            let lbl = if troops_k >= 10.0 {
                format!("{:.0}K", troops_k)
            } else {
                format!("{:.1}K", troops_k)
            };
            map_painter.text(
                badge_rect.center() + Vec2::new(2.5, 0.0),
                egui::Align2::CENTER_CENTER,
                lbl,
                FontId::proportional(if is_selected { 10.5 } else { 9.5 }),
                if is_selected { TEXT_DARK } else { TEXT_PRIMARY },
            );

            unit_hitboxes.push((badge_rect.expand(4.0), unit.id, unit.country_id.clone()));
        }
    }

    if config.show_mode_badge {
        let mode_title = match map_mode {
            MapMode::Political => "MAPA POLÍTICO",
            MapMode::GdpPerCapita => "PIB PER CÁPITA",
            MapMode::Inflation => "RIESGO INFLACIONARIO",
            MapMode::Diplomacy => "DIPLOMACIA",
        };
        let badge_w = (rect.width() - 24.0).clamp(180.0, 310.0);
        let badge_rect =
            Rect::from_min_size(rect.left_top() + Vec2::new(10.0, 10.0), Vec2::new(badge_w, 26.0));
        let mut badge_ui = ui.new_child(egui::UiBuilder::new().max_rect(badge_rect));
        badge_ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
        egui::Frame::new()
            .fill(Color32::from_rgba_unmultiplied(12, 18, 26, 220))
            .stroke(Stroke::new(1.0_f32, BORDER_BRASS))
            .corner_radius(CornerRadius::same(3))
            .inner_margin(Margin::symmetric(8, 3))
            .show(&mut badge_ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(mode_title).strong().size(11.0).color(ACCENT_GOLD));
                    if let Some(sel) = &state.selected_country_id {
                        if let Some(c) = state.countries.get(sel) {
                            ui.label(
                                RichText::new(format!("• {} [{}]", c.name, c.id))
                                    .size(11.0)
                                    .color(TEXT_PRIMARY),
                            );
                        }
                    }
                });
            });
    }

    // Consola táctil flotante de cámara (botón físico estilo HOI4) en la esquina inferior derecha
    let dock_w = (rect.width() - 20.0).clamp(210.0, 248.0);
    let btn_rect = Rect::from_min_size(
        rect.right_bottom()
            - Vec2::new(
                dock_w + config.camera_right_inset,
                31.0 + config.camera_bottom_offset,
            ),
        Vec2::new(dock_w, 31.0),
    );
    let mut ui_child = ui.new_child(egui::UiBuilder::new().max_rect(btn_rect));
    let mut cam_btn_clicked = false;
    let cam_frame = floating_hud_frame(true)
        .inner_margin(Margin::symmetric(5, 2))
        .show(&mut ui_child, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                if step_button(ui, Vec2::new(24.0, 24.0), "", Some(GameIcon::Plus)).clicked() {
                    viewport.camera_3d.zoom_by_factor(1.24);
                    cam_btn_clicked = true;
                }
                if step_button(ui, Vec2::new(24.0, 24.0), "", Some(GameIcon::Minus)).clicked() {
                    viewport.camera_3d.zoom_by_factor(1.0 / 1.24);
                    cam_btn_clicked = true;
                }
                if step_button(ui, Vec2::new(24.0, 24.0), "", Some(GameIcon::RotateLeft)).clicked() {
                    viewport.camera_3d.yaw = (viewport.camera_3d.yaw - 0.18).clamp(-1.35, 1.35);
                    cam_btn_clicked = true;
                }
                if step_button(ui, Vec2::new(24.0, 24.0), "", Some(GameIcon::RotateRight)).clicked() {
                    viewport.camera_3d.yaw = (viewport.camera_3d.yaw + 0.18).clamp(-1.35, 1.35);
                    cam_btn_clicked = true;
                }
                let angle_lbl = if viewport.camera_3d.is_top_down() {
                    "3D"
                } else {
                    "2D"
                };
                if step_button(ui, Vec2::new(44.0, 24.0), angle_lbl, Some(GameIcon::Camera3D)).clicked() {
                    if viewport.camera_3d.is_top_down() {
                        viewport.camera_3d.set_tactical_3d();
                    } else {
                        viewport.camera_3d.set_top_down();
                    }
                    cam_btn_clicked = true;
                }
                if step_button(ui, Vec2::new(62.0, 24.0), "Centro", Some(GameIcon::Center)).clicked() {
                    viewport.camera_3d.reset();
                    cam_btn_clicked = true;
                }
            });
        });
    draw_corner_rivets(ui.painter(), cam_frame.response.rect, BORDER_BRASS);

    // Raycasting 3D exacto o selección de unidad militar al tocar/clicar en el mapa
    if response.clicked() && !cam_btn_clicked {
        if let Some(pointer_pos) = response.interact_pointer_pos() {
            let blocked_by_hud = viewport.hud_rects.iter().any(|r| r.contains(pointer_pos));
            if !btn_rect.contains(pointer_pos) && !blocked_by_hud {
                for (ubox, uid, ucid) in unit_hitboxes.iter().rev() {
                    if ubox.contains(pointer_pos) {
                        return Some(MapPickHit {
                            country_id: ucid.clone(),
                            province_mesh: None,
                            unit_id: Some(*uid),
                        });
                    }
                }
                let (ray_origin, ray_dir) = viewport.camera_3d.screen_to_ray(pointer_pos, rect);
                if let Some((mesh_name, country_id)) = scene.pick_province_at_ray(ray_origin, ray_dir) {
                    return Some(MapPickHit {
                        country_id: country_id.to_string(),
                        province_mesh: Some(mesh_name.to_string()),
                        unit_id: None,
                    });
                }
            }
        }
    }

    let _ = (BG_PANEL, BG_WELL, BORDER_SUBTLE);
    None
}

fn project_world_to_rect(world_pos: [f32; 3], view_proj: glam::Mat4, rect: Rect) -> Option<Pos2> {
    let clip = view_proj * Vec4::new(world_pos[0], world_pos[1], world_pos[2], 1.0);
    if clip.w <= 0.05 {
        return None;
    }
    let ndc = Vec3::new(clip.x / clip.w, clip.y / clip.w, clip.z / clip.w);
    if !(-1.1..=1.1).contains(&ndc.x) || !(-1.1..=1.1).contains(&ndc.y) {
        return None;
    }
    let sx = rect.left() + (ndc.x * 0.5 + 0.5) * rect.width();
    let sy = rect.top() + (0.5 - ndc.y * 0.5) * rect.height();
    Some(Pos2::new(sx, sy))
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
