//! Propósito del archivo:
//! Pantalla de Título / Menú Principal en formato horizontal móvil.
//! Permite iniciar una nueva campaña eligiendo dificultad de la IA local,
//! continuar la partida activa si existe, o abrir la pantalla de ajustes.

use crate::models::{Difficulty, GameState, ScreenRoute};
use crate::ui::theme::{
    ACCENT_CYAN, ACCENT_GOLD, BG_CARD, BG_PANEL, BORDER_BRASS, BORDER_SUBTLE, TEXT_MUTED,
    TEXT_PRIMARY,
};
use egui::{Align, Layout, RichText, Stroke, Ui};

pub enum MainMenuAction {
    None,
    StartNewCampaign(Difficulty),
    ResumeCampaign,
    OpenSettings,
}

pub fn render_main_menu(
    ui: &mut Ui,
    selected_difficulty: &mut Difficulty,
    has_active_campaign: bool,
    state: &GameState,
) -> MainMenuAction {
    let mut action = MainMenuAction::None;

    ui.columns(2, |cols| {
        // Columna izquierda: Identidad visual de campaña y resumen del motor
        cols[0].vertical(|ui| {
            ui.add_space(28.0);
            egui::Frame::new()
                .fill(BG_PANEL)
                .stroke(Stroke::new(1.5_f32, BORDER_BRASS))
                .corner_radius(10.0)
                .inner_margin(24.0)
                .show(ui, |ui| {
                    ui.label(
                        RichText::new("GRAN ESTRATEGIA GEOPOLÍTICA")
                            .size(12.0)
                            .strong()
                            .color(ACCENT_CYAN),
                    );
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new("HORIZONTE LATAM")
                            .size(34.0)
                            .strong()
                            .color(ACCENT_GOLD),
                    );
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new(
                            "Dirige el destino económico, parlamentario, monetario y diplomático \
                             de cualquiera de las 21 naciones de Latinoamérica frente a una \
                             Inteligencia Artificial local paralela.",
                        )
                        .size(14.5)
                        .color(TEXT_PRIMARY),
                    );
                    ui.add_space(18.0);

                    egui::Frame::new()
                        .fill(BG_CARD)
                        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                        .corner_radius(8.0)
                        .inner_margin(12.0)
                        .show(ui, |ui| {
                            ui.label(RichText::new("Capacidades de Campaña").strong().color(ACCENT_GOLD));
                            ui.label("• Mapa vectorial SVG nativo de 21 naciones latinoamericanas");
                            ui.label("• Simulación de Población, Moneda Nacional, Tipo de Cambio y PIB");
                            ui.label("• Congreso multipartidista, elecciones y coaliciones");
                            ui.label("• Motor Lua 5.4 + IA local paralela en Rust (hilo secundario)");
                        });
                });
        });

        // Columna derecha: Botones principales de navegación y selector de dificultad
        cols[1].with_layout(Layout::top_down(Align::Center), |ui| {
            ui.add_space(36.0);
            egui::Frame::new()
                .fill(BG_PANEL)
                .stroke(Stroke::new(1.5_f32, BORDER_SUBTLE))
                .corner_radius(10.0)
                .inner_margin(22.0)
                .show(ui, |ui| {
                    ui.set_min_width(360.0);

                    if has_active_campaign && state.player_country_id.is_some() {
                        if ui
                            .add_sized(
                                [340.0, 50.0],
                                egui::Button::new(
                                    RichText::new("Continuar Campaña Actual")
                                        .size(16.0)
                                        .strong()
                                        .color(ACCENT_GOLD),
                                ),
                            )
                            .clicked()
                        {
                            action = MainMenuAction::ResumeCampaign;
                        }
                        ui.add_space(10.0);
                    }

                    ui.label(RichText::new("Dificultad de la IA Local").strong().color(TEXT_MUTED));
                    ui.horizontal(|ui| {
                        for diff in [Difficulty::Normal, Difficulty::Desafiante, Difficulty::Realista] {
                            let label = match diff {
                                Difficulty::Normal => "Normal",
                                Difficulty::Desafiante => "Desafiante",
                                Difficulty::Realista => "Realista",
                            };
                            if ui.selectable_label(*selected_difficulty == diff, label).clicked() {
                                *selected_difficulty = diff;
                            }
                        }
                    });
                    ui.add_space(12.0);

                    if ui
                        .add_sized(
                            [340.0, 52.0],
                            egui::Button::new(
                                RichText::new("Nueva Campaña (Elegir País)")
                                    .size(16.0)
                                    .strong(),
                            ),
                        )
                        .clicked()
                    {
                        action = MainMenuAction::StartNewCampaign(*selected_difficulty);
                    }

                    ui.add_space(10.0);
                    if ui
                        .add_sized(
                            [340.0, 44.0],
                            egui::Button::new(RichText::new("Ajustes del Juego").size(15.0)),
                        )
                        .clicked()
                    {
                        action = MainMenuAction::OpenSettings;
                    }
                });

            ui.add_space(12.0);
            let _ = ScreenRoute::CountrySelect;
            ui.label(
                RichText::new("Formato horizontal táctil • Rust + Lua 5.4")
                    .size(12.0)
                    .color(TEXT_MUTED),
            );
        });
    });

    action
}
