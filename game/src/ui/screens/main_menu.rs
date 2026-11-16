//! Propósito del archivo:
//! Pantalla de Título / Menú Principal estilo Gran Estrategia (Victoria 2 / HOI4).
//! Presenta placa heráldica del título, resumen táctico de sistemas, selector de dificultad
//! con botones físicos y botones de comando principales para campaña y ajustes.

use crate::models::{Difficulty, GameState, ScreenRoute};
use crate::ui::theme::{
    card_frame, draw_corner_rivets, draw_section_header, game_button, paint_icon, panel_frame,
    well_frame, GameIcon, ACCENT_CYAN, ACCENT_EMERALD, ACCENT_GOLD, BORDER_BRASS,
    BORDER_BRASS_DIM, TEXT_MUTED, TEXT_PRIMARY,
};
use egui::{Color32, CornerRadius, Margin, RichText, Sense, Stroke, Ui, Vec2};

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
            ui.add_space(4.0);
            ui.columns(2, |cols| {
                // Columna izquierda: Placa imperial de campaña y pilares de simulación
                cols[0].vertical(|ui| {
                    let resp = panel_frame(true).inner_margin(14.0).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let (emblem_rect, _) =
                                ui.allocate_exact_size(Vec2::splat(34.0), Sense::hover());
                            ui.painter().rect_filled(
                                emblem_rect,
                                CornerRadius::same(4),
                                Color32::from_rgb(24, 38, 56),
                            );
                            ui.painter().rect_stroke(
                                emblem_rect,
                                CornerRadius::same(4),
                                Stroke::new(1.5_f32, BORDER_BRASS),
                                egui::StrokeKind::Inside,
                            );
                            paint_icon(
                                ui.painter(),
                                emblem_rect.shrink(5.0),
                                GameIcon::Crown,
                                ACCENT_GOLD,
                            );

                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 1.0;
                                ui.label(
                                    RichText::new("HORIZONTE LATAM")
                                        .size(26.0)
                                        .strong()
                                        .color(ACCENT_GOLD),
                                );
                                ui.label(
                                    RichText::new("GRAN ESTRATEGIA GEOPOLÍTICA • 21 NACIONES")
                                        .size(11.0)
                                        .strong()
                                        .color(ACCENT_CYAN),
                                );
                            });
                        });

                        ui.add_space(6.0);
                        well_frame().inner_margin(Margin::symmetric(10, 8)).show(ui, |ui| {
                            ui.label(
                                RichText::new(
                                    "Asume el mando de cualquier nación latinoamericana: equilibra \
                                     moneda e inflación en el Banco Central, negocia el Congreso y \
                                     proyecta tu diplomacia sobre el mapa 3D.",
                                )
                                .size(12.5)
                                .color(TEXT_PRIMARY),
                            );
                        });
                        ui.add_space(8.0);

                        card_frame(Some(BORDER_BRASS_DIM)).show(ui, |ui| {
                            draw_section_header(
                                ui,
                                GameIcon::Dispatch,
                                "SISTEMAS DEL TEATRO REGIONAL",
                                ACCENT_GOLD,
                            );
                            feature_row(
                                ui,
                                GameIcon::Map,
                                "Mapa 3D con relieve andino y raycasting por provincia",
                            );
                            feature_row(
                                ui,
                                GameIcon::Economy,
                                "Economía real: PIB, divisas, inflación y deuda pública",
                            );
                            feature_row(
                                ui,
                                GameIcon::Politics,
                                "Congreso multipartidista, elecciones y gobernabilidad",
                            );
                            feature_row(
                                ui,
                                GameIcon::Military,
                                "IA regional paralela en Rust + reglas modulares Lua 5.4",
                            );
                        });
                    });
                    draw_corner_rivets(ui.painter(), resp.response.rect, BORDER_BRASS);
                });

                // Columna derecha: Centro de Mando con botones físicos
                cols[1].vertical(|ui| {
                    let resp = panel_frame(true).inner_margin(14.0).show(ui, |ui| {
                        let inner_w = ui.available_width();
                        draw_section_header(
                            ui,
                            GameIcon::Military,
                            "CUARTEL GENERAL DE CAMPAÑA",
                            ACCENT_CYAN,
                        );

                        if has_active_campaign && state.player_country_id.is_some() {
                            if game_button(
                                ui,
                                Vec2::new(inner_w, 44.0),
                                "CONTINUAR CAMPAÑA ACTIVA",
                                Some(GameIcon::Play),
                                true,
                                Some(ACCENT_EMERALD),
                            )
                            .clicked()
                            {
                                action = MainMenuAction::ResumeCampaign;
                            }
                            ui.add_space(8.0);
                        }

                        ui.label(
                            RichText::new("DIFICULTAD DE LA IA LOCAL")
                                .size(11.5)
                                .strong()
                                .color(TEXT_MUTED),
                        );
                        ui.add_space(3.0);

                        let seg_w = ((inner_w - 14.0) / 3.0).max(74.0);
                        ui.horizontal(|ui| {
                            for diff in [
                                Difficulty::Normal,
                                Difficulty::Desafiante,
                                Difficulty::Realista,
                            ] {
                                let active = *selected_difficulty == diff;
                                let label = match diff {
                                    Difficulty::Normal => "Normal",
                                    Difficulty::Desafiante => "Desafiante",
                                    Difficulty::Realista => "Realista",
                                };
                                if game_button(
                                    ui,
                                    Vec2::new(seg_w, 36.0),
                                    label,
                                    None,
                                    active,
                                    Some(ACCENT_CYAN),
                                )
                                .clicked()
                                {
                                    *selected_difficulty = diff;
                                }
                            }
                        });

                        ui.add_space(12.0);

                        if game_button(
                            ui,
                            Vec2::new(inner_w, 48.0),
                            "NUEVA CAMPAÑA — ELEGIR NACIÓN",
                            Some(GameIcon::Crown),
                            true,
                            Some(ACCENT_GOLD),
                        )
                        .clicked()
                        {
                            action = MainMenuAction::StartNewCampaign(*selected_difficulty);
                        }

                        ui.add_space(8.0);

                        if game_button(
                            ui,
                            Vec2::new(inner_w, 40.0),
                            "AJUSTES Y ESTADO DEL MOTOR",
                            Some(GameIcon::Settings),
                            false,
                            None,
                        )
                        .clicked()
                        {
                            action = MainMenuAction::OpenSettings;
                        }

                        ui.add_space(10.0);
                        let _ = ScreenRoute::CountrySelect;
                        well_frame().show(ui, |ui| {
                            ui.label(
                                RichText::new(
                                    "Mando táctil horizontal • Pulsa un botón o desliza paneles",
                                )
                                .size(11.0)
                                .color(TEXT_MUTED),
                            );
                        });
                    });
                    draw_corner_rivets(ui.painter(), resp.response.rect, BORDER_BRASS);
                });
            });
            ui.add_space(6.0);
        });

    action
}

fn feature_row(ui: &mut Ui, icon: GameIcon, text: &str) {
    ui.horizontal(|ui| {
        let (ic_rect, _) = ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
        paint_icon(ui.painter(), ic_rect, icon, ACCENT_CYAN);
        ui.label(RichText::new(text).size(12.0).color(TEXT_PRIMARY));
    });
}
