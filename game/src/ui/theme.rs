//! Propósito del archivo:
//! Define la paleta visual táctica (azul pizarra oscuro, oro diplomático, bordes nítidos),
//! tipografía escalada para pantallas móviles horizontales y formateadores numéricos
//! para población, monedas nacionales y PIB.

use crate::models::CountryState;
use egui::{Color32, CornerRadius, Margin, Stroke, Style, Visuals};

pub const BG_DEEP: Color32 = Color32::from_rgb(11, 20, 32);
pub const BG_PANEL: Color32 = Color32::from_rgb(19, 32, 48);
pub const BG_CARD: Color32 = Color32::from_rgb(26, 43, 64);
pub const BORDER_BRASS: Color32 = Color32::from_rgb(184, 146, 74);
pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(47, 74, 104);
pub const ACCENT_GOLD: Color32 = Color32::from_rgb(235, 189, 88);
pub const ACCENT_EMERALD: Color32 = Color32::from_rgb(56, 189, 128);
pub const ACCENT_CRIMSON: Color32 = Color32::from_rgb(229, 72, 77);
pub const ACCENT_CYAN: Color32 = Color32::from_rgb(76, 187, 232);
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(238, 243, 248);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(154, 174, 196);

/// Aplica el estilo táctico de escritorio de mando al contexto `egui`.
pub fn apply_tactical_theme(ctx: &egui::Context) {
    let mut style: Style = (*ctx.style()).clone();
    let mut visuals = Visuals::dark();

    visuals.window_fill = BG_PANEL;
    visuals.panel_fill = BG_DEEP;
    visuals.faint_bg_color = BG_CARD;
    visuals.extreme_bg_color = Color32::from_rgb(7, 13, 22);
    visuals.widgets.noninteractive.bg_fill = BG_CARD;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, BORDER_SUBTLE);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT_PRIMARY);

    visuals.widgets.inactive.bg_fill = Color32::from_rgb(30, 50, 74);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER_SUBTLE);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(7);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT_PRIMARY);

    visuals.widgets.hovered.bg_fill = Color32::from_rgb(44, 72, 106);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.5_f32, ACCENT_GOLD);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(7);

    visuals.widgets.active.bg_fill = Color32::from_rgb(56, 90, 130);
    visuals.widgets.active.bg_stroke = Stroke::new(1.5_f32, ACCENT_GOLD);
    visuals.widgets.active.corner_radius = CornerRadius::same(7);

    visuals.selection.bg_fill = Color32::from_rgb(24, 96, 138);
    visuals.selection.stroke = Stroke::new(1.5_f32, ACCENT_CYAN);

    style.visuals = visuals;
    // Crítico en móvil: evita que arrastrar el dedo sobre textos seleccione texto en lugar de hacer scroll
    style.interaction.selectable_labels = false;
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(14.0, 9.0);
    style.spacing.window_margin = Margin::same(12);
    style.spacing.scroll.bar_width = 8.0;
    style.spacing.scroll.handle_min_length = 28.0;
    ctx.set_style(style);
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
