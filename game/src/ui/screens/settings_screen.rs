//! Propósito del archivo:
//! Pantalla dedicada de Opciones y Estado del Motor (`Settings`).
//! Permite cambiar dificultad, reiniciar campaña o volver a seleccionar otra nación.

use crate::models::{Difficulty, GameState};
use crate::ui::theme::{ACCENT_CYAN, ACCENT_GOLD, BG_PANEL, BORDER_SUBTLE};
use egui::{RichText, Stroke, Ui};

pub enum SettingsAction {
    None,
    ReturnToMap,
    ReturnToMainMenu,
    RestartSelectCountry,
}

pub fn render_settings_screen(
    ui: &mut Ui,
    state: &mut GameState,
    has_3d_scene: bool,
    use_3d_map: &mut bool,
) -> SettingsAction {
    let mut action = SettingsAction::None;

    egui::ScrollArea::vertical()
        .id_salt("settings_screen_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Frame::new()
                .fill(BG_PANEL)
                .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                .corner_radius(8.0)
                .inner_margin(16.0)
                .show(ui, |ui| {
                    ui.label(
                        RichText::new("OPCIONES DE CAMPAÑA Y MOTOR")
                            .strong()
                            .size(18.0)
                            .color(ACCENT_GOLD),
                    );
                    ui.separator();
                    ui.add_space(6.0);

                    ui.label(RichText::new("Dificultad de la IA Local (Rust + Rayon)").strong());
                    ui.horizontal(|ui| {
                        for (diff, lbl) in [
                            (Difficulty::Normal, "Normal"),
                            (Difficulty::Desafiante, "Desafiante"),
                            (Difficulty::Realista, "Realista"),
                        ] {
                            if ui.selectable_label(state.difficulty == diff, lbl).clicked() {
                                state.difficulty = diff;
                            }
                        }
                    });

                    ui.add_space(10.0);
                    ui.label(RichText::new("Modo del Visor Cartográfico").strong());
                    ui.horizontal(|ui| {
                        if has_3d_scene {
                            if ui.selectable_label(*use_3d_map, "Mapa 3D (wgpu)").clicked() {
                                *use_3d_map = true;
                            }
                        }
                        if ui.selectable_label(!*use_3d_map, "Mapa 2D (SVG táctil)").clicked() {
                            *use_3d_map = false;
                        }
                    });

                    ui.add_space(12.0);
                    ui.label(
                        RichText::new("Arquitectura: Interfaz Rust (egui/eframe + wgpu) + Lua 5.4 + NativeActivity Android")
                            .color(ACCENT_CYAN),
                    );
                    ui.label("Soporte nativo ABI: 64-bit (arm64-v8a / x86_64) y 32-bit (armeabi-v7a).");
                    ui.add_space(14.0);

                    ui.horizontal_wrapped(|ui| {
                        if state.player_country_id.is_some()
                            && ui.add_sized([185.0, 40.0], egui::Button::new("Volver al Mapa")).clicked()
                        {
                            action = SettingsAction::ReturnToMap;
                        }
                        if ui
                            .add_sized([205.0, 40.0], egui::Button::new("Elegir Otra Nación"))
                            .clicked()
                        {
                            action = SettingsAction::RestartSelectCountry;
                        }
                        if ui
                            .add_sized([195.0, 40.0], egui::Button::new("Ir al Menú Principal"))
                            .clicked()
                        {
                            action = SettingsAction::ReturnToMainMenu;
                        }
                    });
                });
        });

    action
}
