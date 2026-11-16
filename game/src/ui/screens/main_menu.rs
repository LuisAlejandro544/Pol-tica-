//! Propósito del archivo:
//! Pantalla de Título / Menú Principal en formato horizontal móvil.
//! Permite desplazarse verticalmente en pantallas compactas, elegir dificultad de la
//! IA local, iniciar una nueva campaña, continuar la partida activa o abrir ajustes.

use crate::models::{Difficulty, GameState, ScreenRoute};
use crate::ui::theme::{
    ACCENT_CYAN, ACCENT_EMERALD, ACCENT_GOLD, BG_CARD, BG_PANEL, BORDER_BRASS, BORDER_SUBTLE,
    TEXT_MUTED, TEXT_PRIMARY,
};
use egui::{Color32, RichText, Stroke, Ui};

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

    egui::ScrollArea::vertical()
        .id_salt("main_menu_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(6.0);
            ui.columns(2, |cols| {
                // Columna izquierda: Identidad visual de campaña, KPIs del motor y resumen
                cols[0].vertical(|ui| {
                    egui::Frame::new()
                        .fill(BG_PANEL)
                        .stroke(Stroke::new(1.5_f32, BORDER_BRASS))
                        .corner_radius(10.0)
                        .inner_margin(18.0)
                        .show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                egui::Frame::new()
                                    .fill(Color32::from_rgb(20, 56, 82))
                                    .stroke(Stroke::new(1.0_f32, ACCENT_CYAN))
                                    .corner_radius(4.0)
                                    .inner_margin(egui::Margin::symmetric(8, 3))
                                    .show(ui, |ui| {
                                        ui.label(
                                            RichText::new("GRAN ESTRATEGIA GEOPOLÍTICA")
                                                .size(11.0)
                                                .strong()
                                                .color(ACCENT_CYAN),
                                        );
                                    });
                                egui::Frame::new()
                                    .fill(Color32::from_rgb(48, 38, 18))
                                    .stroke(Stroke::new(1.0_f32, BORDER_BRASS))
                                    .corner_radius(4.0)
                                    .inner_margin(egui::Margin::symmetric(8, 3))
                                    .show(ui, |ui| {
                                        ui.label(
                                            RichText::new("21 NACIONES")
                                                .size(11.0)
                                                .strong()
                                                .color(ACCENT_GOLD),
                                        );
                                    });
                            });

                            ui.add_space(8.0);
                            ui.label(
                                RichText::new("HORIZONTE LATAM")
                                    .size(30.0)
                                    .strong()
                                    .color(ACCENT_GOLD),
                            );
                            ui.add_space(6.0);
                            ui.label(
                                RichText::new(
                                    "Dirige la economía, moneda, congreso y diplomacia de cualquier \
                                     nación latinoamericana frente a una IA regional paralela.",
                                )
                                .size(13.5)
                                .color(TEXT_PRIMARY),
                            );
                            ui.add_space(12.0);

                            // Tarjeta de capacidades del simulador
                            egui::Frame::new()
                                .fill(BG_CARD)
                                .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                                .corner_radius(8.0)
                                .inner_margin(12.0)
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new("Sistemas de Simulación")
                                            .strong()
                                            .color(ACCENT_GOLD),
                                    );
                                    ui.add_space(4.0);
                                    feature_row(ui, "Mapa 3D con relieve andino y provincias interactivas");
                                    feature_row(ui, "Economía real: PIB, inflación, tasas y tipo de cambio");
                                    feature_row(ui, "Congreso multipartidista, elecciones y gobernabilidad");
                                    feature_row(ui, "Motor nativo Rust + wgpu + Lua 5.4 (IA en hilo paralelo)");
                                });
                        });
                });

                // Columna derecha: Centro de Comando (Dificultad y Acciones principales)
                cols[1].vertical(|ui| {
                    egui::Frame::new()
                        .fill(BG_PANEL)
                        .stroke(Stroke::new(1.5_f32, BORDER_SUBTLE))
                        .corner_radius(10.0)
                        .inner_margin(18.0)
                        .show(ui, |ui| {
                            let inner_w = ui.available_width();
                            ui.label(
                                RichText::new("CENTRO DE COMANDO")
                                    .size(13.0)
                                    .strong()
                                    .color(ACCENT_CYAN),
                            );
                            ui.separator();
                            ui.add_space(4.0);

                            if has_active_campaign && state.player_country_id.is_some() {
                                let resume_btn = egui::Button::new(
                                    RichText::new("Continuar Campaña Actual")
                                        .size(15.5)
                                        .strong()
                                        .color(ACCENT_EMERALD),
                                )
                                .stroke(Stroke::new(1.5_f32, ACCENT_EMERALD));
                                if ui.add_sized([inner_w, 46.0], resume_btn).clicked() {
                                    action = MainMenuAction::ResumeCampaign;
                                }
                                ui.add_space(10.0);
                            }

                            ui.label(
                                RichText::new("Dificultad de la IA Local")
                                    .size(13.0)
                                    .strong()
                                    .color(TEXT_MUTED),
                            );
                            ui.add_space(4.0);

                            // Selector segmentado claro y táctil
                            let seg_w = ((inner_w - 16.0) / 3.0).max(70.0);
                            ui.horizontal(|ui| {
                                for diff in [Difficulty::Normal, Difficulty::Desafiante, Difficulty::Realista] {
                                    let active = *selected_difficulty == diff;
                                    let label = match diff {
                                        Difficulty::Normal => "Normal",
                                        Difficulty::Desafiante => "Desafiante",
                                        Difficulty::Realista => "Realista",
                                    };
                                    let btn = egui::Button::new(
                                        RichText::new(label).size(13.0).strong().color(if active {
                                            Color32::from_rgb(15, 22, 32)
                                        } else {
                                            TEXT_PRIMARY
                                        }),
                                    )
                                    .fill(if active { ACCENT_CYAN } else { BG_CARD })
                                    .stroke(Stroke::new(
                                        1.0_f32,
                                        if active { ACCENT_CYAN } else { BORDER_SUBTLE },
                                    ));
                                    if ui.add_sized([seg_w, 36.0], btn).clicked() {
                                        *selected_difficulty = diff;
                                    }
                                }
                            });

                            ui.add_space(14.0);

                            let start_btn = egui::Button::new(
                                RichText::new("Nueva Campaña  —  Elegir País >")
                                    .size(16.0)
                                    .strong()
                                    .color(Color32::from_rgb(15, 22, 32)),
                            )
                            .fill(ACCENT_GOLD)
                            .stroke(Stroke::new(1.5_f32, ACCENT_GOLD));

                            if ui.add_sized([inner_w, 50.0], start_btn).clicked() {
                                action = MainMenuAction::StartNewCampaign(*selected_difficulty);
                            }

                            ui.add_space(10.0);

                            let settings_btn = egui::Button::new(
                                RichText::new("Ajustes y Estado del Motor")
                                    .size(14.5)
                                    .color(TEXT_PRIMARY),
                            )
                            .fill(BG_CARD)
                            .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE));

                            if ui.add_sized([inner_w, 42.0], settings_btn).clicked() {
                                action = MainMenuAction::OpenSettings;
                            }

                            ui.add_space(12.0);
                            ui.separator();
                            let _ = ScreenRoute::CountrySelect;
                            ui.label(
                                RichText::new("Formato horizontal táctil • Desliza para explorar cada pantalla")
                                    .size(11.5)
                                    .color(TEXT_MUTED),
                            );
                        });
                });
            });
            ui.add_space(8.0);
        });

    action
}

fn feature_row(ui: &mut Ui, text: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("▸").strong().color(ACCENT_CYAN));
        ui.label(RichText::new(text).size(12.5).color(TEXT_PRIMARY));
    });
}
