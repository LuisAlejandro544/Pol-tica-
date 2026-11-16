//! Propósito del archivo:
//! Sistema visual de Gran Estrategia (inspirado en mesas de mando Victoria 2 / HOI4):
//! paleta de acero oscuro y latón antiguo, marcos biselados con remaches en las esquinas,
//! escudos heráldicos procedurales por nación, íconos vectoriales nítidos dibujados con `Painter`,
//! botones táctiles metálicos y barras de medición para estabilidad, economía y diplomacia.

use crate::models::CountryState;
use egui::{
    Align2, Color32, CornerRadius, FontId, Margin, Pos2, Rect, Response, RichText, Sense, Stroke,
    Ui, Vec2,
};

pub const BG_DEEP: Color32 = Color32::from_rgb(10, 15, 22);
pub const BG_PANEL: Color32 = Color32::from_rgb(18, 26, 36);
pub const BG_CARD: Color32 = Color32::from_rgb(24, 34, 48);
pub const BG_WELL: Color32 = Color32::from_rgb(12, 18, 26);
pub const BG_BTN: Color32 = Color32::from_rgb(33, 47, 66);
pub const BG_BTN_HOVER: Color32 = Color32::from_rgb(47, 66, 92);
pub const BG_BTN_ACTIVE: Color32 = Color32::from_rgb(198, 154, 68);

pub const BORDER_BRASS: Color32 = Color32::from_rgb(194, 152, 74);
pub const BORDER_BRASS_DIM: Color32 = Color32::from_rgb(112, 88, 46);
pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(54, 74, 100);
pub const BEVEL_LIGHT: Color32 = Color32::from_rgb(86, 114, 148);

pub const ACCENT_GOLD: Color32 = Color32::from_rgb(240, 198, 102);
pub const ACCENT_EMERALD: Color32 = Color32::from_rgb(64, 196, 134);
pub const ACCENT_CRIMSON: Color32 = Color32::from_rgb(220, 74, 78);
pub const ACCENT_CYAN: Color32 = Color32::from_rgb(86, 192, 236);
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(238, 240, 234);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(158, 173, 192);
pub const TEXT_DARK: Color32 = Color32::from_rgb(16, 20, 26);

/// Margen derecho reservado para evitar que la barra de navegación Android (`↩ ○ ☰`) tape botones.
pub const SAFE_RIGHT_PAD: i8 = 40;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameIcon {
    Map,
    Economy,
    Politics,
    Diplomacy,
    Military,
    Settings,
    Play,
    Pause,
    NextTurn,
    Crown,
    Population,
    Treasury,
    Currency,
    Growth,
    Stability,
    Plus,
    Minus,
    RotateLeft,
    RotateRight,
    Camera3D,
    Center,
    Dispatch,
}

/// Aplica la estética de Gran Estrategia (esquinas mecanizadas, alto contraste, botones con cuerpo) al contexto `egui`.
pub fn apply_tactical_theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    let mut visuals = egui::Visuals::dark();

    visuals.window_fill = BG_PANEL;
    visuals.panel_fill = BG_DEEP;
    visuals.faint_bg_color = BG_CARD;
    visuals.extreme_bg_color = BG_WELL;

    visuals.widgets.noninteractive.bg_fill = BG_CARD;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, BORDER_SUBTLE);
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(4);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT_PRIMARY);

    visuals.widgets.inactive.bg_fill = BG_BTN;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER_SUBTLE);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(4);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT_PRIMARY);

    visuals.widgets.hovered.bg_fill = BG_BTN_HOVER;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.5_f32, ACCENT_GOLD);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(4);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, TEXT_PRIMARY);

    visuals.widgets.active.bg_fill = Color32::from_rgb(62, 86, 118);
    visuals.widgets.active.bg_stroke = Stroke::new(1.5_f32, ACCENT_GOLD);
    visuals.widgets.active.corner_radius = CornerRadius::same(4);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, ACCENT_GOLD);

    visuals.selection.bg_fill = Color32::from_rgb(36, 94, 134);
    visuals.selection.stroke = Stroke::new(1.5_f32, ACCENT_GOLD);

    style.visuals = visuals;
    // Crítico en móvil: evita que arrastrar el dedo sobre textos seleccione texto en lugar de hacer scroll
    style.interaction.selectable_labels = false;
    style.spacing.item_spacing = egui::vec2(7.0, 7.0);
    style.spacing.button_padding = egui::vec2(10.0, 7.0);
    style.spacing.window_margin = Margin::same(10);
    style.spacing.scroll.bar_width = 7.0;
    style.spacing.scroll.handle_min_length = 28.0;
    ctx.set_style(style);
}

pub fn panel_frame(brass: bool) -> egui::Frame {
    egui::Frame::new()
        .fill(BG_PANEL)
        .stroke(Stroke::new(
            if brass { 1.5_f32 } else { 1.0_f32 },
            if brass { BORDER_BRASS } else { BORDER_SUBTLE },
        ))
        .corner_radius(CornerRadius::same(4))
        .inner_margin(10.0)
}

pub fn card_frame(accent_border: Option<Color32>) -> egui::Frame {
    egui::Frame::new()
        .fill(BG_CARD)
        .stroke(Stroke::new(1.0_f32, accent_border.unwrap_or(BORDER_SUBTLE)))
        .corner_radius(CornerRadius::same(3))
        .inner_margin(8.0)
}

pub fn well_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(BG_WELL)
        .stroke(Stroke::new(1.0_f32, BORDER_BRASS_DIM))
        .corner_radius(CornerRadius::same(3))
        .inner_margin(Margin::symmetric(8, 5))
}

/// Dibuja remaches de latón en las cuatro esquinas de un panel de mando.
pub fn draw_corner_rivets(painter: &egui::Painter, rect: Rect, color: Color32) {
    let r = 2.0_f32;
    let pad = 6.0_f32;
    for pt in [
        Pos2::new(rect.left() + pad, rect.top() + pad),
        Pos2::new(rect.right() - pad, rect.top() + pad),
        Pos2::new(rect.left() + pad, rect.bottom() - pad),
        Pos2::new(rect.right() - pad, rect.bottom() - pad),
    ] {
        painter.circle_filled(pt, r, color);
    }
}

/// Encabezado estilo Victoria 2 / HOI4 con ícono vectorial y filete dorado inferior.
pub fn draw_section_header(ui: &mut Ui, icon: GameIcon, title: &str, color: Color32) {
    ui.horizontal(|ui| {
        let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
        paint_icon(ui.painter(), icon_rect, icon, color);
        ui.label(RichText::new(title).strong().size(13.5).color(color));
    });
    let line_w = ui.available_width().max(24.0);
    let (line_rect, _) = ui.allocate_exact_size(Vec2::new(line_w, 3.0), Sense::hover());
    ui.painter().hline(
        line_rect.x_range(),
        line_rect.center().y,
        Stroke::new(1.0_f32, BORDER_BRASS_DIM),
    );
    ui.add_space(3.0);
}

/// Botón táctil mecanizado estilo HOI4/Victoria 2 con bisel superior, borde nítido e ícono opcional.
pub fn game_button(
    ui: &mut Ui,
    size: Vec2,
    label: &str,
    icon: Option<GameIcon>,
    active: bool,
    accent: Option<Color32>,
) -> Response {
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    if ui.is_rect_visible(rect) {
        let hovered = response.hovered();
        let pressed = response.is_pointer_button_down_on();
        let active_col = accent.unwrap_or(BG_BTN_ACTIVE);

        let fill = if active {
            active_col
        } else if pressed {
            Color32::from_rgb(56, 80, 110)
        } else if hovered {
            BG_BTN_HOVER
        } else {
            BG_BTN
        };

        let border_col = if active {
            ACCENT_GOLD
        } else if hovered {
            BORDER_BRASS
        } else if let Some(ac) = accent {
            ac.gamma_multiply(0.75)
        } else {
            BORDER_SUBTLE
        };

        let p = ui.painter();
        p.rect_filled(rect, CornerRadius::same(4), fill);
        p.rect_stroke(
            rect,
            CornerRadius::same(4),
            Stroke::new(if active || hovered { 1.5_f32 } else { 1.0_f32 }, border_col),
            egui::StrokeKind::Inside,
        );

        // Bisel superior metálico
        let top_y = rect.top() + 1.5;
        let bevel_col = if active {
            Color32::from_rgba_unmultiplied(255, 245, 210, 110)
        } else {
            Color32::from_rgba_unmultiplied(150, 185, 225, 45)
        };
        p.hline((rect.left() + 3.0)..=(rect.right() - 3.0), top_y, Stroke::new(1.0_f32, bevel_col));

        let fg = if active { TEXT_DARK } else { TEXT_PRIMARY };
        let font_size = if size.y >= 38.0 { 13.5 } else { 12.5 };

        if let Some(ic) = icon {
            let ic_size = (size.y * 0.48).clamp(12.0, 17.0);
            if label.is_empty() {
                let ic_rect = Rect::from_center_size(rect.center(), Vec2::splat(ic_size));
                paint_icon(p, ic_rect, ic, fg);
            } else {
                let ic_rect = Rect::from_center_size(
                    Pos2::new(rect.left() + 14.0 + ic_size * 0.35, rect.center().y),
                    Vec2::splat(ic_size),
                );
                paint_icon(p, ic_rect, ic, if active { TEXT_DARK } else { accent.unwrap_or(ACCENT_GOLD) });
                let text_pos = Pos2::new(ic_rect.right() + 7.0, rect.center().y);
                p.text(
                    text_pos,
                    Align2::LEFT_CENTER,
                    label,
                    FontId::proportional(font_size),
                    fg,
                );
            }
        } else {
            p.text(
                rect.center(),
                Align2::CENTER_CENTER,
                label,
                FontId::proportional(font_size),
                fg,
            );
        }
    }
    response
}

/// Botón compacto cuadrado/rectangular para controles `-` / `+`, cámara y pasos rápidos.
pub fn step_button(ui: &mut Ui, size: Vec2, label: &str, icon: Option<GameIcon>) -> Response {
    game_button(ui, size, label, icon, false, None)
}

/// Barra de progreso táctica (Estabilidad, Aprestamiento, Opinión, Deuda).
pub fn draw_gauge_bar(
    ui: &mut Ui,
    width: f32,
    height: f32,
    ratio: f32,
    fill_color: Color32,
    overlay_text: &str,
) {
    let ratio = ratio.clamp(0.0, 1.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width.max(40.0), height), Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let p = ui.painter();
    p.rect_filled(rect, CornerRadius::same(3), BG_WELL);

    let fill_w = (rect.width() - 2.0) * ratio;
    if fill_w > 1.0 {
        let fill_rect = Rect::from_min_size(
            Pos2::new(rect.left() + 1.0, rect.top() + 1.0),
            Vec2::new(fill_w, (rect.height() - 2.0).max(2.0)),
        );
        p.rect_filled(fill_rect, CornerRadius::same(2), fill_color);
    }

    p.rect_stroke(
        rect,
        CornerRadius::same(3),
        Stroke::new(1.0_f32, BORDER_BRASS_DIM),
        egui::StrokeKind::Inside,
    );
    if !overlay_text.is_empty() {
        p.text(
            rect.center(),
            Align2::CENTER_CENTER,
            overlay_text,
            FontId::proportional((height * 0.65).clamp(10.5, 13.0)),
            TEXT_PRIMARY,
        );
    }
}

/// Dibuja un escudo heráldico / insignia de país (inspirado en escudos de Victoria 2 / HOI4).
pub fn draw_country_shield(ui: &mut Ui, size: Vec2, country: &CountryState) {
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let p = ui.painter();
    let base = country_base_color(country);
    let dark = Color32::from_rgb(
        (base.r() as u16 * 55 / 100) as u8,
        (base.g() as u16 * 55 / 100) as u8,
        (base.b() as u16 * 55 / 100) as u8,
    );
    let light = Color32::from_rgb(
        (base.r() as u16 + 45).min(255) as u8,
        (base.g() as u16 + 45).min(255) as u8,
        (base.b() as u16 + 45).min(255) as u8,
    );

    p.rect_filled(rect, CornerRadius::same(3), dark);

    // Banda superior y banda central estilizada
    let top_band = Rect::from_min_max(
        rect.min,
        Pos2::new(rect.max.x, rect.top() + rect.height() * 0.36),
    );
    p.rect_filled(top_band, CornerRadius::same(2), base);

    let mid_band = Rect::from_min_max(
        Pos2::new(rect.left(), rect.top() + rect.height() * 0.36),
        Pos2::new(rect.max.x, rect.top() + rect.height() * 0.64),
    );
    p.rect_filled(mid_band, CornerRadius::ZERO, light);

    p.rect_stroke(
        rect,
        CornerRadius::same(3),
        Stroke::new(1.5_f32, BORDER_BRASS),
        egui::StrokeKind::Inside,
    );

    p.text(
        rect.center(),
        Align2::CENTER_CENTER,
        &country.id,
        FontId::proportional((size.y * 0.44).clamp(9.5, 14.0)),
        TEXT_PRIMARY,
    );
}

/// Dibuja íconos vectoriales nítidos sin depender de fuentes emoji externas.
pub fn paint_icon(p: &egui::Painter, rect: Rect, icon: GameIcon, color: Color32) {
    let c = rect.center();
    let r = rect.width().min(rect.height()) * 0.42;
    let stroke = Stroke::new(1.5_f32, color);

    match icon {
        GameIcon::Map => {
            p.circle_stroke(c, r, stroke);
            p.hline((c.x - r)..=(c.x + r), c.y, stroke);
            p.vline(c.x, (c.y - r)..=(c.y + r), stroke);
        }
        GameIcon::Economy | GameIcon::Treasury => {
            p.circle_stroke(c, r, stroke);
            p.circle_filled(c, r * 0.32, color);
        }
        GameIcon::Politics | GameIcon::Crown => {
            let pts = vec![
                Pos2::new(c.x - r, c.y + r * 0.65),
                Pos2::new(c.x - r, c.y - r * 0.35),
                Pos2::new(c.x - r * 0.35, c.y + r * 0.1),
                Pos2::new(c.x, c.y - r * 0.75),
                Pos2::new(c.x + r * 0.35, c.y + r * 0.1),
                Pos2::new(c.x + r, c.y - r * 0.35),
                Pos2::new(c.x + r, c.y + r * 0.65),
            ];
            p.line_segment([pts[0], pts[1]], stroke);
            p.line_segment([pts[1], pts[2]], stroke);
            p.line_segment([pts[2], pts[3]], stroke);
            p.line_segment([pts[3], pts[4]], stroke);
            p.line_segment([pts[4], pts[5]], stroke);
            p.line_segment([pts[5], pts[6]], stroke);
            p.line_segment([pts[6], pts[0]], stroke);
        }
        GameIcon::Diplomacy => {
            p.circle_stroke(Pos2::new(c.x - r * 0.32, c.y), r * 0.62, stroke);
            p.circle_stroke(Pos2::new(c.x + r * 0.32, c.y), r * 0.62, stroke);
        }
        GameIcon::Military | GameIcon::Stability => {
            let pts = vec![
                Pos2::new(c.x - r * 0.8, c.y - r * 0.8),
                Pos2::new(c.x + r * 0.8, c.y - r * 0.8),
                Pos2::new(c.x + r * 0.8, c.y + r * 0.1),
                Pos2::new(c.x, c.y + r * 0.95),
                Pos2::new(c.x - r * 0.8, c.y + r * 0.1),
            ];
            for i in 0..pts.len() {
                p.line_segment([pts[i], pts[(i + 1) % pts.len()]], stroke);
            }
        }
        GameIcon::Settings => {
            p.circle_stroke(c, r * 0.52, stroke);
            for i in 0..6 {
                let a = (i as f32) * std::f32::consts::TAU / 6.0;
                let p0 = Pos2::new(c.x + a.cos() * r * 0.55, c.y + a.sin() * r * 0.55);
                let p1 = Pos2::new(c.x + a.cos() * r * 0.95, c.y + a.sin() * r * 0.95);
                p.line_segment([p0, p1], stroke);
            }
        }
        GameIcon::Play => {
            p.add(egui::Shape::convex_polygon(
                vec![
                    Pos2::new(c.x - r * 0.55, c.y - r * 0.78),
                    Pos2::new(c.x + r * 0.85, c.y),
                    Pos2::new(c.x - r * 0.55, c.y + r * 0.78),
                ],
                color,
                Stroke::NONE,
            ));
        }
        GameIcon::Pause => {
            let bar_w = (r * 0.42).max(2.0);
            p.rect_filled(
                Rect::from_center_size(Pos2::new(c.x - r * 0.42, c.y), Vec2::new(bar_w, r * 1.5)),
                CornerRadius::same(1),
                color,
            );
            p.rect_filled(
                Rect::from_center_size(Pos2::new(c.x + r * 0.42, c.y), Vec2::new(bar_w, r * 1.5)),
                CornerRadius::same(1),
                color,
            );
        }
        GameIcon::NextTurn => {
            for dx in [-r * 0.35, r * 0.30] {
                p.line_segment(
                    [Pos2::new(c.x + dx - r * 0.3, c.y - r * 0.65), Pos2::new(c.x + dx + r * 0.35, c.y)],
                    stroke,
                );
                p.line_segment(
                    [Pos2::new(c.x + dx + r * 0.35, c.y), Pos2::new(c.x + dx - r * 0.3, c.y + r * 0.65)],
                    stroke,
                );
            }
        }
        GameIcon::Population => {
            p.circle_filled(Pos2::new(c.x, c.y - r * 0.45), r * 0.36, color);
            p.line_segment(
                [Pos2::new(c.x - r * 0.65, c.y + r * 0.75), Pos2::new(c.x, c.y + r * 0.05)],
                stroke,
            );
            p.line_segment(
                [Pos2::new(c.x, c.y + r * 0.05), Pos2::new(c.x + r * 0.65, c.y + r * 0.75)],
                stroke,
            );
        }
        GameIcon::Currency => {
            p.rect_stroke(
                Rect::from_center_size(c, Vec2::new(r * 1.8, r * 1.25)),
                CornerRadius::same(2),
                stroke,
                egui::StrokeKind::Inside,
            );
            p.circle_filled(c, r * 0.28, color);
        }
        GameIcon::Growth => {
            p.line_segment(
                [Pos2::new(c.x - r * 0.8, c.y + r * 0.6), Pos2::new(c.x - r * 0.1, c.y)],
                stroke,
            );
            p.line_segment(
                [Pos2::new(c.x - r * 0.1, c.y), Pos2::new(c.x + r * 0.2, c.y + r * 0.25)],
                stroke,
            );
            p.line_segment(
                [Pos2::new(c.x + r * 0.2, c.y + r * 0.25), Pos2::new(c.x + r * 0.85, c.y - r * 0.65)],
                stroke,
            );
        }
        GameIcon::Plus => {
            p.hline((c.x - r * 0.75)..=(c.x + r * 0.75), c.y, stroke);
            p.vline(c.x, (c.y - r * 0.75)..=(c.y + r * 0.75), stroke);
        }
        GameIcon::Minus => {
            p.hline((c.x - r * 0.75)..=(c.x + r * 0.75), c.y, stroke);
        }
        GameIcon::RotateLeft => {
            p.circle_stroke(c, r * 0.72, stroke);
            p.line_segment(
                [Pos2::new(c.x - r * 0.9, c.y - r * 0.1), Pos2::new(c.x - r * 0.5, c.y + r * 0.3)],
                stroke,
            );
        }
        GameIcon::RotateRight => {
            p.circle_stroke(c, r * 0.72, stroke);
            p.line_segment(
                [Pos2::new(c.x + r * 0.9, c.y - r * 0.1), Pos2::new(c.x + r * 0.5, c.y + r * 0.3)],
                stroke,
            );
        }
        GameIcon::Camera3D => {
            p.rect_stroke(
                Rect::from_center_size(c, Vec2::new(r * 1.4, r * 1.4)),
                CornerRadius::same(1),
                stroke,
                egui::StrokeKind::Inside,
            );
            p.line_segment(
                [Pos2::new(c.x - r * 0.7, c.y - r * 0.7), Pos2::new(c.x + r * 0.7, c.y + r * 0.7)],
                stroke,
            );
        }
        GameIcon::Center => {
            p.circle_stroke(c, r * 0.62, stroke);
            p.circle_filled(c, r * 0.22, color);
        }
        GameIcon::Dispatch => {
            p.rect_stroke(
                Rect::from_center_size(c, Vec2::new(r * 1.5, r * 1.65)),
                CornerRadius::same(1),
                stroke,
                egui::StrokeKind::Inside,
            );
            p.hline((c.x - r * 0.45)..=(c.x + r * 0.45), c.y - r * 0.3, stroke);
            p.hline((c.x - r * 0.45)..=(c.x + r * 0.45), c.y + r * 0.2, stroke);
        }
    }
}

pub fn format_population(pop: u64) -> String {
    if pop >= 1_000_000 {
        format!("{:.2} M", (pop as f64) / 1_000_000.0)
    } else if pop >= 1_000 {
        format!("{:.1} K", (pop as f64) / 1_000.0)
    } else {
        format!("{pop}")
    }
}

pub fn format_usd_millions(millions: f64) -> String {
    if millions >= 1_000_000.0 {
        format!("${:.2} T", millions / 1_000_000.0)
    } else if millions >= 1_000.0 {
        format!("${:.1} B", millions / 1_000.0)
    } else {
        format!("${:.0} M", millions)
    }
}

pub fn ideology_color(ideology: &str) -> Color32 {
    match ideology {
        "Izquierda" | "Socialista" => Color32::from_rgb(214, 69, 69),
        "Centroizquierda" | "Progresista" => Color32::from_rgb(235, 117, 74),
        "Centro" | "Reformista" => Color32::from_rgb(235, 189, 88),
        "Centroderecha" | "Libertario" => Color32::from_rgb(76, 166, 232),
        "Derecha" | "Conservador" => Color32::from_rgb(52, 99, 189),
        "Ecologista" => Color32::from_rgb(56, 189, 128),
        _ => Color32::from_rgb(152, 124, 212),
    }
}

pub fn country_base_color(country: &CountryState) -> Color32 {
    Color32::from_rgb(country.color[0], country.color[1], country.color[2])
}
