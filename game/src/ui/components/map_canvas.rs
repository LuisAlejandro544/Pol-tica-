//! Propósito del archivo:
//! Componente interactivo reutilizable que renderiza el mapa vectorial `latinoamerica.svg`
//! teselado en GPU dentro de `egui`, permitiendo paneo táctil, zoom, modos de mapa
//! (Político, PIB per cápita, Inflación, Diplomacia) y selección directa por toque.

use crate::map::LatamVectorMap;
use crate::models::{GameState, MapMode};
use crate::ui::theme::{
    ACCENT_CRIMSON, ACCENT_CYAN, ACCENT_EMERALD, ACCENT_GOLD, BG_DEEP, BORDER_SUBTLE, TEXT_PRIMARY,
};
use egui::{Color32, FontId, Mesh, Pos2, Rect, Sense, Shape, Stroke, Ui, Vec2};

pub struct MapViewportState {
    pub zoom: f32,
    pub pan: Vec2,
}

impl Default for MapViewportState {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: Vec2::ZERO,
        }
    }
}

/// Dibuja el mapa SVG de Latinoamérica y devuelve el ID del país tocado si hubo clic/tap.
pub fn render_latam_map(
    ui: &mut Ui,
    vector_map: &LatamVectorMap,
    state: &GameState,
    map_mode: MapMode,
    viewport: &mut MapViewportState,
) -> Option<String> {
    let available = ui.available_size();
    let (rect, response) = ui.allocate_exact_size(available, Sense::click_and_drag());

    // Fondo oceánico y marco cartográfico
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 8.0, Color32::from_rgb(10, 26, 44));
    painter.rect_stroke(
        rect,
        8.0,
        Stroke::new(1.5, BORDER_SUBTLE),
        egui::StrokeKind::Inside,
    );

    // Manejo de paneo táctil / arrastre y zoom con rueda o pellizco
    if response.dragged() {
        viewport.pan += response.drag_delta();
    }
    let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);
    if response.hovered() && scroll_delta.abs() > 0.1 {
        let factor = (1.0 + scroll_delta * 0.002).clamp(0.8, 1.25);
        viewport.zoom = (viewport.zoom * factor).clamp(0.75, 4.5);
    }

    // Cálculo de escala para encajar el viewBox (1000x1200) dentro del rectángulo horizontal
    let base_scale = (rect.width() / vector_map.width)
        .min(rect.height() / vector_map.height)
        * 0.94;
    let scale = base_scale * viewport.zoom;
    let map_pixel_size = Vec2::new(vector_map.width * scale, vector_map.height * scale);
    let origin = rect.center() - map_pixel_size * 0.5 + viewport.pan;

    let to_screen = |svg_p: Pos2| -> Pos2 { Pos2::new(origin.x + svg_p.x * scale, origin.y + svg_p.y * scale) };
    let to_svg = |scr_p: Pos2| -> Pos2 { Pos2::new((scr_p.x - origin.x) / scale, (scr_p.y - origin.y) / scale) };

    // Retícula sutil geográfica
    let grid_color = Color32::from_rgba_unmultiplied(38, 78, 115, 65);
    for gx in (0..=1000).step_by(100) {
        let p0 = to_screen(Pos2::new(gx as f32, 0.0));
        let p1 = to_screen(Pos2::new(gx as f32, vector_map.height));
        painter.line_segment([p0, p1], Stroke::new(0.8, grid_color));
    }
    for gy in (0..=1200).step_by(100) {
        let p0 = to_screen(Pos2::new(0.0, gy as f32));
        let p1 = to_screen(Pos2::new(vector_map.width, gy as f32));
        painter.line_segment([p0, p1], Stroke::new(0.8, grid_color));
    }

    // Dibujar cada país teselado desde el SVG
    for (id, mesh_data) in &vector_map.countries {
        let is_selected = state.selected_country_id.as_deref() == Some(id.as_str());
        let is_player = state.player_country_id.as_deref() == Some(id.as_str());

        let fill = compute_country_fill(id, state, map_mode, is_selected, is_player);

        // Construir malla GPU con triángulos pre-teselados por lyon_tessellation
        let mut mesh = Mesh::default();
        for &v in &mesh_data.triangles_vertices {
            mesh.colored_vertex(to_screen(v), fill);
        }
        mesh.indices.extend_from_slice(&mesh_data.triangles_indices);
        painter.add(Shape::mesh(mesh));

        // Contorno nacional (resaltado si es el país seleccionado o el jugador)
        let screen_outline: Vec<Pos2> = mesh_data.outline.iter().map(|&p| to_screen(p)).collect();
        let stroke = if is_selected {
            Stroke::new(2.6, ACCENT_GOLD)
        } else if is_player {
            Stroke::new(2.2, ACCENT_CYAN)
        } else {
            Stroke::new(1.1, BG_DEEP)
        };
        painter.add(Shape::closed_line(screen_outline, stroke));

        // Etiqueta ISO o nombre en países grandes o cuando el zoom aumenta
        let bbox_w = (mesh_data.bbox_max.x - mesh_data.bbox_min.x) * scale;
        if bbox_w > 22.0 || is_selected || is_player {
            let label_pos = to_screen(mesh_data.centroid);
            let label = if bbox_w > 78.0 {
                mesh_data.display_name.as_str()
            } else {
                id.as_str()
            };
            painter.text(
                label_pos,
                egui::Align2::CENTER_CENTER,
                label,
                FontId::proportional((10.5 * viewport.zoom.sqrt()).clamp(9.0, 15.0)),
                TEXT_PRIMARY,
            );
        }
    }

    // Controles flotantes de zoom táctil en esquina inferior derecha del mapa
    let btn_rect = Rect::from_min_size(rect.right_bottom() - Vec2::new(136.0, 44.0), Vec2::new(124.0, 34.0));
    let mut ui_child = ui.new_child(egui::UiBuilder::new().max_rect(btn_rect));
    ui_child.horizontal(|ui| {
        if ui.small_button(" + ").clicked() {
            viewport.zoom = (viewport.zoom * 1.22).min(4.5);
        }
        if ui.small_button(" - ").clicked() {
            viewport.zoom = (viewport.zoom / 1.22).max(0.75);
        }
        if ui.small_button("Centrar").clicked() {
            viewport.zoom = 1.0;
            viewport.pan = Vec2::ZERO;
        }
    });

    // Detección de toque/clic sobre un país
    if response.clicked() {
        if let Some(pointer_pos) = response.interact_pointer_pos() {
            let svg_pos = to_svg(pointer_pos);
            if let Some(hit_id) = vector_map.hit_test(svg_pos) {
                return Some(hit_id.to_string());
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
                    lerp_color(Color32::from_rgb(160, 64, 64), Color32::from_rgb(64, 155, 108), t)
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
