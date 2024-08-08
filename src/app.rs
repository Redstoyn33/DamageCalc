use crate::calc::{Calc, Stats, Unit};
use eframe::epaint::text::TextWrapMode;
use eframe::epaint::FontFamily;
use egui::epaint::PathStroke;
use egui::{
    lerp, Align2, Color32, FontId, Rangef, RichText, Rounding, Sense, Stroke, TextStyle, Ui, Vec2,
    Widget,
};
use std::mem::swap;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
pub struct DamageCalcApp {
    calc: Calc,
    teams: Vec<Team>,
    team0: usize,
    team1: usize,

    json_classes: String,
    json_errs: i32,
    json_window: bool,
    units_count: usize,
    settings_window: bool,
    negative_stats: bool,
    can_kill_yourself: bool,

    style: Style,
    #[serde(skip)]
    damages: Vec<DamageEffect>,
}

impl Default for DamageCalcApp {
    fn default() -> Self {
        Self {
            calc: Default::default(),
            teams: Vec::new(),
            team0: 0,
            team1: 0,
            json_classes: "".to_string(),
            json_errs: -1,
            json_window: false,
            units_count: 0,
            settings_window: false,
            negative_stats: false,
            can_kill_yourself: false,
            style: Default::default(),
            damages: vec![],
        }
    }
}

impl DamageCalcApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        if let Some(storage) = cc.storage {
            if let Some(state) = eframe::get_value::<Self>(storage, eframe::APP_KEY) {
                state.style.apply_mono(&cc.egui_ctx);
                return state;
            }
        }
        Default::default()
    }
}

impl eframe::App for DamageCalcApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut reset_all = false;
        egui::Window::new("settings")
            .open(&mut self.settings_window)
            .show(ctx, |ui| {
                ui.collapsing("style settings", |ui| {
                    ui.checkbox(&mut self.style.fancy_stats, "fancy stats");
                    egui::Slider::new(&mut self.style.box_size, 0.0..=100.)
                        .clamp_to_range(false)
                        .ui(ui);
                    egui::Slider::new(&mut self.style.line_size, 0.0..=20.)
                        .clamp_to_range(false)
                        .ui(ui);
                    if ui
                        .toggle_value(&mut self.style.mono, "toggle mono text")
                        .changed()
                    {
                        self.style.apply_mono(ctx);
                    }
                    egui::ScrollArea::horizontal()
                        .stick_to_right(true)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let mut delete_color = None;
                                for (i, box_color) in self.style.box_colors.iter_mut().enumerate() {
                                    ui.vertical(|ui| {
                                        ui.color_edit_button_srgba(box_color);
                                        if ui.button("-").clicked() {
                                            delete_color = Some(i);
                                        }
                                    });
                                }
                                if let Some(delete_color) = delete_color {
                                    self.style.box_colors.remove(delete_color);
                                }
                                if ui.button("+").clicked() {
                                    self.style.box_colors.push(
                                        self.style
                                            .box_colors
                                            .last()
                                            .unwrap_or(&Color32::default())
                                            .clone(),
                                    );
                                }
                            });
                        });
                });
                ui.collapsing("real settings", |ui| {
                    egui::Label::new(RichText::new("⚠ memory loss warning ⚠").heading())
                        .wrap_mode(TextWrapMode::Extend)
                        .ui(ui);
                    ui.separator();
                    ui.checkbox(&mut self.negative_stats, "negative stats");
                    ui.checkbox(&mut self.can_kill_yourself, "can kill yourself");
                    ui.separator();
                    if egui::DragValue::new(&mut self.units_count)
                        .range(0..=33)
                        .clamp_to_range(false)
                        .suffix(" units in team")
                        .ui(ui)
                        .changed()
                    {
                        for team in self.teams.iter_mut() {
                            team.units.resize_with(self.units_count, || None);
                        }
                    }
                    if ui.button("add team").clicked() {
                        self.teams.push(Team::new(self.units_count));
                    }
                    egui::ScrollArea::horizontal().show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let mut remove_index = None;
                            for (i, team) in self.teams.iter_mut().enumerate() {
                                ui.vertical(|ui| {
                                    ui.set_max_width(130.);
                                    ui.text_edit_singleline(&mut team.name);
                                    if ui.button("clear units").clicked() {
                                        for unit in team.units.iter_mut() {
                                            *unit = None;
                                        }
                                    }
                                    if ui.button("clear units stats").clicked() {
                                        for unit in team.units.iter_mut() {
                                            if let Some(unit) = unit {
                                                unit.stats = Stats::default();
                                                unit.value = 0;
                                            }
                                        }
                                    }
                                    if ui.button("delete").clicked() {
                                        remove_index = Some(i);
                                    }
                                });
                            }
                            if let Some(i) = remove_index {
                                self.teams.remove(i);
                            }
                        });
                    });
                    ui.separator();
                    if ui.button("⚠ reset all app ⚠").clicked() {
                        reset_all = true;
                    }
                });
            });
        if reset_all {
            *self = DamageCalcApp::default();
        }
        egui::Window::new("json editor")
            .open(&mut self.json_window)
            .max_height(ctx.available_rect().height() - 100.)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("apply").clicked() {
                        self.json_errs = self.calc.update(&self.json_classes);
                    }
                    ui.label(format!("errors: {}", self.json_errs));
                });
                ui.separator();
                ui.label("invalid units:");
                egui::ScrollArea::vertical()
                    .id_source("invalid_units")
                    .max_height(ctx.available_rect().height() / 2.)
                    .show(ui, |ui| {
                        for (team_i, team) in self.teams.iter_mut().enumerate() {
                            for (unit_i, opt_unit) in team.units.iter_mut().enumerate() {
                                if let Some(unit) = opt_unit {
                                    if !self.calc.classes.contains_key(&unit.name) {
                                        let unit_name = &unit.name.clone();
                                        ui.horizontal(|ui| {
                                            if ui
                                                .button(format!(
                                                    "{}_{}#{}",
                                                    &team.name, unit_i, unit_name
                                                ))
                                                .clicked()
                                            {
                                                if self.team0 == team_i {
                                                } else if self.team1 == team_i {
                                                    swap(&mut self.team0, &mut self.team1);
                                                } else {
                                                    self.team0 = team_i;
                                                }
                                                team.select = unit_i;
                                            }
                                            if ui.button("remove").clicked() {
                                                *opt_unit = None;
                                            }
                                        });
                                    }
                                }
                            }
                        }
                    });
                ui.separator();
                egui::ScrollArea::vertical()
                    .id_source("json_editor")
                    .show(ui, |ui| {
                        egui::TextEdit::multiline(&mut self.json_classes)
                            .code_editor()
                            .desired_width(f32::INFINITY)
                            .show(ui);
                    });
            });
        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("settings").clicked() {
                    self.settings_window = !self.settings_window;
                }
                if ui
                    .button(if self.json_errs == 0 {
                        "json".to_string()
                    } else {
                        format!("json {} errs", self.json_errs)
                    })
                    .clicked()
                {
                    self.json_window = !self.json_window;
                }
            });
        });
        egui::SidePanel::left("team0_panel")
            .resizable(false)
            .show(ctx, |ui| {
                egui::ComboBox::from_id_source("team0_select")
                    .selected_text(&self.teams.get(self.team0).unwrap_or(&Team::new(0)).name)
                    .show_ui(ui, |ui| {
                        for (i, team) in self.teams.iter().enumerate() {
                            ui.selectable_value(&mut self.team0, i, &team.name);
                        }
                    });
                if let Some(team) = self.teams.get_mut(self.team0) {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        if let Some(sel) = DamageCalcApp::select_column(ui, team, &self.style) {
                            team.select = sel;
                        }
                    });
                } else {
                    self.team0 = 0;
                }
            });
        egui::SidePanel::right("team1_panel")
            .resizable(false)
            .show(ctx, |ui| {
                egui::ComboBox::from_id_source("team1_select")
                    .selected_text(&self.teams.get(self.team1).unwrap_or(&Team::new(0)).name)
                    .show_ui(ui, |ui| {
                        for (i, team) in self.teams.iter().enumerate() {
                            ui.selectable_value(&mut self.team1, i, &team.name);
                        }
                    });
                if let Some(team) = self.teams.get_mut(self.team1) {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        if let Some(sel) = DamageCalcApp::select_column(ui, team, &self.style) {
                            team.select = sel;
                        }
                    });
                } else {
                    self.team0 = 0;
                }
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            let (mut team0_panel, mut team1_panel) =
                ui.max_rect().split_left_right_at_fraction(0.5);
            ui.painter().vline(
                team0_panel.max.x,
                Rangef::new(team0_panel.min.y, team0_panel.max.y),
                PathStroke::new(1., ui.style().visuals.window_stroke.color),
            );
            team0_panel.max.x -= ui.spacing().window_margin.right;
            team1_panel.min.x += ui.spacing().window_margin.left;
            let mut team_render = |ui: &mut Ui,
                                   team: &mut Team,
                                   team_num: usize,
                                   enemy_team: &mut Team| {
                if let Some(unit_cell) = team.units.get_mut(team.select) {
                    if let Some(unit) = unit_cell {
                        ui.horizontal(|ui| {
                            ui.add_space((team0_panel.width() - ui.spacing().combo_width) / 2.);
                            egui::ComboBox::from_id_source(format!("{}_unit_class", team_num))
                                .selected_text(&unit.name)
                                .show_ui(ui, |ui| {
                                    for class in self.calc.classes.keys() {
                                        if class.eq(&unit.name) {
                                            continue;
                                        }
                                        ui.selectable_value(&mut unit.name, class.clone(), class);
                                    }
                                });
                        });
                        if self.style.fancy_stats {
                            ui.horizontal(|ui| {
                                ui.columns(6, |ui| {
                                    ui[0].vertical_centered(|ui| {
                                        ui.label("atk:");
                                    });
                                    ui[2].vertical_centered(|ui| {
                                        ui.label("+");
                                    });
                                    ui[4].vertical_centered(|ui| {
                                        ui.label("=");
                                    });
                                    ui[3].vertical_centered(|ui| {
                                        egui::DragValue::new(&mut unit.stats.attack)
                                            .range(
                                                if self.negative_stats { i32::MIN } else { 0 }
                                                    ..=i32::MAX,
                                            )
                                            .ui(ui);
                                    });
                                    if let Some(base_stats) = self.calc.classes.get(&unit.name) {
                                        ui[1].vertical_centered(|ui| {
                                            ui.label(base_stats.attack.to_string());
                                        });
                                        ui[5].vertical_centered(|ui| {
                                            ui.label(
                                                (base_stats.attack + unit.stats.attack).to_string(),
                                            );
                                        });
                                    } else {
                                        ui[1].vertical_centered(|ui| {
                                            ui.label("-");
                                        });
                                        ui[5].vertical_centered(|ui| {
                                            ui.label("-");
                                        });
                                    }
                                });
                            });
                            ui.horizontal(|ui| {
                                ui.columns(6, |ui| {
                                    ui[0].vertical_centered(|ui| {
                                        ui.label("def:");
                                    });
                                    ui[2].vertical_centered(|ui| {
                                        ui.label("+");
                                    });
                                    ui[4].vertical_centered(|ui| {
                                        ui.label("=");
                                    });
                                    ui[3].vertical_centered(|ui| {
                                        egui::DragValue::new(&mut unit.stats.defense)
                                            .range(
                                                if self.negative_stats { i32::MIN } else { 0 }
                                                    ..=i32::MAX,
                                            )
                                            .ui(ui);
                                    });
                                    if let Some(base_stats) = self.calc.classes.get(&unit.name) {
                                        ui[1].vertical_centered(|ui| {
                                            ui.label(base_stats.defense.to_string());
                                        });
                                        ui[5].vertical_centered(|ui| {
                                            ui.label(
                                                (base_stats.defense + unit.stats.defense)
                                                    .to_string(),
                                            );
                                        });
                                    } else {
                                        ui[1].vertical_centered(|ui| {
                                            ui.label("-");
                                        });
                                        ui[5].vertical_centered(|ui| {
                                            ui.label("-");
                                        });
                                    }
                                });
                            });
                            ui.horizontal(|ui| {
                                ui.columns(6, |ui| {
                                    ui[0].vertical_centered(|ui| {
                                        ui.label("hp:");
                                    });
                                    ui[2].vertical_centered(|ui| {
                                        ui.label("+");
                                    });
                                    ui[4].vertical_centered(|ui| {
                                        ui.label("=");
                                    });
                                    ui[3].vertical_centered(|ui| {
                                        egui::DragValue::new(&mut unit.stats.health)
                                            .range(
                                                if self.negative_stats { i32::MIN } else { 0 }
                                                    ..=i32::MAX,
                                            )
                                            .ui(ui);
                                    });
                                    if let Some(base_stats) = self.calc.classes.get(&unit.name) {
                                        ui[1].vertical_centered(|ui| {
                                            ui.label(base_stats.health.to_string());
                                        });
                                        ui[5].vertical_centered(|ui| {
                                            ui.label(
                                                (base_stats.health + unit.stats.health).to_string(),
                                            );
                                        });
                                    } else {
                                        ui[1].vertical_centered(|ui| {
                                            ui.label("-");
                                        });
                                        ui[5].vertical_centered(|ui| {
                                            ui.label("-");
                                        });
                                    }
                                });
                            });
                            ui.horizontal(|ui| {
                                ui.columns(6, |ui| {
                                    ui[0].vertical_centered(|ui| {
                                        ui.label("dmg_:");
                                    });
                                    ui[2].vertical_centered(|ui| {
                                        ui.label("+");
                                    });
                                    ui[4].vertical_centered(|ui| {
                                        ui.label("=");
                                    });
                                    ui[3].vertical_centered(|ui| {
                                        egui::DragValue::new(&mut unit.stats.min_dmg)
                                            .range(
                                                if self.negative_stats { i32::MIN } else { 0 }
                                                    ..=i32::MAX,
                                            )
                                            .ui(ui);
                                    });
                                    if let Some(base_stats) = self.calc.classes.get(&unit.name) {
                                        ui[1].vertical_centered(|ui| {
                                            ui.label(base_stats.min_dmg.to_string());
                                        });
                                        ui[5].vertical_centered(|ui| {
                                            ui.label(
                                                (base_stats.min_dmg + unit.stats.min_dmg).to_string(),
                                            );
                                        });
                                    } else {
                                        ui[1].vertical_centered(|ui| {
                                            ui.label("-");
                                        });
                                        ui[5].vertical_centered(|ui| {
                                            ui.label("-");
                                        });
                                    }
                                });
                            });
                            ui.horizontal(|ui| {
                                ui.columns(6, |ui| {
                                    ui[0].vertical_centered(|ui| {
                                        ui.label("dmg[]:");
                                    });
                                    ui[2].vertical_centered(|ui| {
                                        ui.label("+");
                                    });
                                    ui[4].vertical_centered(|ui| {
                                        ui.label("=");
                                    });
                                    ui[3].vertical_centered(|ui| {
                                        egui::DragValue::new(&mut unit.stats.max_dmg)
                                            .range(
                                                if self.negative_stats { i32::MIN } else { 0 }
                                                    ..=i32::MAX,
                                            )
                                            .ui(ui);
                                    });
                                    if let Some(base_stats) = self.calc.classes.get(&unit.name) {
                                        ui[1].vertical_centered(|ui| {
                                            ui.label(base_stats.max_dmg.to_string());
                                        });
                                        ui[5].vertical_centered(|ui| {
                                            ui.label(
                                                (base_stats.max_dmg + unit.stats.max_dmg).to_string(),
                                            );
                                        });
                                    } else {
                                        ui[1].vertical_centered(|ui| {
                                            ui.label("-");
                                        });
                                        ui[5].vertical_centered(|ui| {
                                            ui.label("-");
                                        });
                                    }
                                });
                            });
                            ui.horizontal(|ui| {
                                ui.columns(3, |ui| {
                                    egui::DragValue::new(&mut unit.value)
                                        .range(0..=i32::MAX)
                                        .suffix(" count")
                                        .ui(&mut ui[0]);
                                    egui::DragValue::new(&mut team.percent)
                                        .range(0..=i32::MAX)
                                        .suffix("%")
                                        .ui(&mut ui[1]);
                                    ui[2].checkbox(&mut team.retaliation, "retaliation")
                                });
                            });
                            ui.vertical_centered_justified(|ui| {
                                if let Some(base_stats) = self.calc.classes.get(&unit.name) {
                                    egui::ProgressBar::new(
                                        1. - unit.damage_left as f32
                                            / (base_stats.health + unit.stats.health) as f32,
                                    )
                                    .text(format!(
                                        "{}/{}",
                                        base_stats.health + unit.stats.health - unit.damage_left,
                                        base_stats.health + unit.stats.health
                                    ))
                                    .ui(ui);
                                } else {
                                    egui::ProgressBar::new(1.).text("-").ui(ui);
                                }
                            });
                            ui.vertical_centered_justified(|ui| {
                                match (
                                    team.units.get_mut(team.select),
                                    enemy_team.units.get_mut(enemy_team.select),
                                ) {
                                    (Some(Some(unit)), Some(Some(enemy_unit))) => {
                                        if ui.button("attack").clicked() {
                                            let (dmg, self_dmg) = self.calc.calculate(
                                                enemy_unit,
                                                unit,
                                                team.percent,
                                                enemy_team.retaliation,
                                            );
                                            self.damages.push(DamageEffect::new(dmg));
                                            if let Some(dmg) = self_dmg {
                                                self.damages.push(DamageEffect::new(dmg));
                                            }
                                        }
                                    }
                                    _ => {
                                        ui.add_enabled(false, egui::Button::new("attack"));
                                    }
                                }
                                if ui
                                    .button("remove")
                                    .on_hover_text("middle click")
                                    .middle_clicked()
                                {
                                    if let Some(delete_unit) = team.units.get_mut(team.select) {
                                        *delete_unit = None;
                                    }
                                }
                                if self.can_kill_yourself {
                                    ui.columns(2, |ui| {
                                        let sel = team.second_select;
                                        egui::DragValue::new(&mut team.second_select)
                                            .range(0..=team.units.len() - 1)
                                            .suffix(format!(
                                                " {}",
                                                &team
                                                    .units
                                                    .get(sel)
                                                    .cloned()
                                                    .unwrap_or(None)
                                                    .unwrap_or(Unit {
                                                        name: "-".to_string(),
                                                        stats: Default::default(),
                                                        value: 0,
                                                        damage_left: 0,
                                                    })
                                                    .name
                                            ))
                                            .ui(&mut ui[1]);
                                        if ui[0].button("attack yourself").clicked() {
                                            let mut u1 = team.units.clone();
                                            let mut u2 = team.units.clone();
                                            match (
                                                u1.get_mut(team.select),
                                                u2.get_mut(team.second_select),
                                            ) {
                                                (Some(Some(unit)), Some(Some(enemy_unit))) => {
                                                    let (dmg, self_dmg) = self.calc.calculate(
                                                        enemy_unit,
                                                        unit,
                                                        team.percent,
                                                        team.retaliation,
                                                    );
                                                    *team.units.get_mut(team.select).unwrap() =
                                                        Some(unit.clone());
                                                    *team
                                                        .units
                                                        .get_mut(team.second_select)
                                                        .unwrap() = Some(enemy_unit.clone());
                                                    self.damages.push(DamageEffect::new(dmg));
                                                    if let Some(dmg) = self_dmg {
                                                        self.damages.push(DamageEffect::new(dmg));
                                                    }
                                                }
                                                _ => {}
                                            }
                                        }
                                    });
                                }
                            });
                        } else {
                            ui.horizontal(|ui| {
                                ui.columns(4, |ui| {
                                    ui[0].label("atk:");
                                    egui::DragValue::new(&mut unit.stats.attack)
                                        .range(
                                            if self.negative_stats { i32::MIN } else { 0 }
                                                ..=i32::MAX,
                                        )
                                        .ui(&mut ui[2]);
                                    if let Some(base_stats) = self.calc.classes.get(&unit.name) {
                                        ui[1].label(base_stats.attack.to_string());
                                        ui[3].label(
                                            (base_stats.attack + unit.stats.attack).to_string(),
                                        );
                                    } else {
                                        ui[1].label("-");
                                        ui[3].label("-");
                                    }
                                });
                            });
                            ui.horizontal(|ui| {
                                ui.columns(4, |ui| {
                                    ui[0].label("def:");
                                    egui::DragValue::new(&mut unit.stats.defense)
                                        .range(
                                            if self.negative_stats { i32::MIN } else { 0 }
                                                ..=i32::MAX,
                                        )
                                        .ui(&mut ui[2]);
                                    if let Some(base_stats) = self.calc.classes.get(&unit.name) {
                                        ui[1].label(base_stats.defense.to_string());
                                        ui[3].label(
                                            (base_stats.defense + unit.stats.defense).to_string(),
                                        );
                                    } else {
                                        ui[1].label("-");
                                        ui[3].label("-");
                                    }
                                });
                            });
                            ui.horizontal(|ui| {
                                ui.columns(4, |ui| {
                                    ui[0].label("hp:");
                                    egui::DragValue::new(&mut unit.stats.health)
                                        .range(
                                            if self.negative_stats { i32::MIN } else { 0 }
                                                ..=i32::MAX,
                                        )
                                        .ui(&mut ui[2]);
                                    if let Some(base_stats) = self.calc.classes.get(&unit.name) {
                                        ui[1].label(base_stats.health.to_string());
                                        ui[3].label(
                                            (base_stats.health + unit.stats.health).to_string(),
                                        );
                                    } else {
                                        ui[1].label("-");
                                        ui[3].label("-");
                                    }
                                });
                            });
                            ui.horizontal(|ui| {
                                ui.columns(4, |ui| {
                                    ui[0].label("dmg_:");
                                    egui::DragValue::new(&mut unit.stats.min_dmg)
                                        .range(
                                            if self.negative_stats { i32::MIN } else { 0 }
                                                ..=i32::MAX,
                                        )
                                        .ui(&mut ui[2]);
                                    if let Some(base_stats) = self.calc.classes.get(&unit.name) {
                                        ui[1].label(base_stats.min_dmg.to_string());
                                        ui[3].label(
                                            (base_stats.min_dmg + unit.stats.min_dmg).to_string(),
                                        );
                                    } else {
                                        ui[1].label("-");
                                        ui[3].label("-");
                                    }
                                });
                            });
                            ui.horizontal(|ui| {
                                ui.columns(4, |ui| {
                                    ui[0].label("dmg[]:");
                                    egui::DragValue::new(&mut unit.stats.max_dmg)
                                        .range(
                                            if self.negative_stats { i32::MIN } else { 0 }
                                                ..=i32::MAX,
                                        )
                                        .ui(&mut ui[2]);
                                    if let Some(base_stats) = self.calc.classes.get(&unit.name) {
                                        ui[1].label(base_stats.max_dmg.to_string());
                                        ui[3].label(
                                            (base_stats.max_dmg + unit.stats.max_dmg).to_string(),
                                        );
                                    } else {
                                        ui[1].label("-");
                                        ui[3].label("-");
                                    }
                                });
                            });
                            ui.horizontal(|ui| {
                                ui.columns(4, |ui| {
                                    egui::DragValue::new(&mut unit.value)
                                        .range(0..=i32::MAX)
                                        .clamp_to_range(false)
                                        .suffix(" count")
                                        .ui(&mut ui[0]);
                                    egui::DragValue::new(&mut team.percent)
                                        .range(0..=i32::MAX)
                                        .clamp_to_range(false)
                                        .suffix("%")
                                        .ui(&mut ui[1]);
                                    ui[2].checkbox(&mut team.retaliation, "retaliation")
                                });
                            });
                            if let Some(base_stats) = self.calc.classes.get(&unit.name) {
                                egui::ProgressBar::new(
                                    1. - unit.damage_left as f32
                                        / (base_stats.health + unit.stats.health) as f32,
                                )
                                .text(format!(
                                    "{}/{}",
                                    base_stats.health + unit.stats.health - unit.damage_left,
                                    base_stats.health + unit.stats.health
                                ))
                                .ui(ui);
                            } else {
                                egui::ProgressBar::new(1.).text("-").ui(ui);
                            }
                            ui.horizontal(|ui| {
                                ui.columns(2, |ui| {
                                    match (
                                        team.units.get_mut(team.select),
                                        enemy_team.units.get_mut(enemy_team.select),
                                    ) {
                                        (Some(Some(unit)), Some(Some(enemy_unit))) => {
                                            if ui[0].button("attack").clicked() {
                                                let (dmg, self_dmg) = self.calc.calculate(
                                                    enemy_unit,
                                                    unit,
                                                    team.percent,
                                                    enemy_team.retaliation,
                                                );
                                                self.damages.push(DamageEffect::new(dmg));
                                                if let Some(dmg) = self_dmg {
                                                    self.damages.push(DamageEffect::new(dmg));
                                                }
                                            }
                                        }
                                        _ => {
                                            ui[0].add_enabled(false, egui::Button::new("attack"));
                                        }
                                    }
                                    if ui[1]
                                        .button("remove")
                                        .on_hover_text("middle click")
                                        .middle_clicked()
                                    {
                                        if let Some(delete_unit) = team.units.get_mut(team.select) {
                                            *delete_unit = None;
                                        }
                                    }
                                });
                            });
                            if self.can_kill_yourself {
                                ui.horizontal(|ui| {
                                    ui.columns(2, |ui| {
                                        let sel = team.second_select;
                                        egui::DragValue::new(&mut team.second_select)
                                            .range(0..=team.units.len() - 1)
                                            .suffix(format!(
                                                " {}",
                                                &team
                                                    .units
                                                    .get(sel)
                                                    .cloned()
                                                    .unwrap_or(None)
                                                    .unwrap_or(Unit {
                                                        name: "-".to_string(),
                                                        stats: Default::default(),
                                                        value: 0,
                                                        damage_left: 0,
                                                    })
                                                    .name
                                            ))
                                            .ui(&mut ui[1]);
                                        if ui[0].button("attack yourself").clicked() {
                                            let mut u1 = team.units.clone();
                                            let mut u2 = team.units.clone();
                                            match (
                                                u1.get_mut(team.select),
                                                u2.get_mut(team.second_select),
                                            ) {
                                                (Some(Some(unit)), Some(Some(enemy_unit))) => {
                                                    let (dmg, self_dmg) = self.calc.calculate(
                                                        enemy_unit,
                                                        unit,
                                                        team.percent,
                                                        team.retaliation,
                                                    );
                                                    *team.units.get_mut(team.select).unwrap() =
                                                        Some(unit.clone());
                                                    *team
                                                        .units
                                                        .get_mut(team.second_select)
                                                        .unwrap() = Some(enemy_unit.clone());
                                                    self.damages.push(DamageEffect::new(dmg));
                                                    if let Some(dmg) = self_dmg {
                                                        self.damages.push(DamageEffect::new(dmg));
                                                    }
                                                }
                                                _ => {}
                                            }
                                        }
                                    });
                                });
                            }
                        }
                        if let Some(unit_cell) = team.units.get_mut(team.select) {
                            if let Some(unit) = unit_cell {
                                if let Some(base) = self.calc.classes.get(&unit.name) {
                                    ui.label(&base.desc);
                                }
                            }
                        }
                    } else {
                        ui.horizontal(|ui| {
                            let mut new_class = None;
                            ui.add_space((team0_panel.width() - ui.spacing().combo_width) / 2.);
                            egui::ComboBox::from_id_source(format!("{}_unit_class", team_num))
                                .selected_text("💀")
                                .show_ui(ui, |ui| {
                                    for class in self.calc.classes.keys() {
                                        ui.selectable_value(&mut new_class, Some(class), class);
                                    }
                                });
                            if let Some(class) = new_class {
                                *unit_cell = Some(Unit {
                                    name: class.clone(),
                                    stats: Default::default(),
                                    value: 0,
                                    damage_left: 0,
                                });
                            }
                        });
                    }
                }
            };
            ui.allocate_ui_at_rect(team0_panel, |ui| {
                if self.team0 != self.team1 && self.team0.max(self.team1) < self.teams.len() {
                    let (l, r) = self.teams.split_at_mut(self.team0.min(self.team1) + 1);
                    match (
                        l.last_mut(),
                        r.get_mut(self.team0.max(self.team1) - self.team0.min(self.team1) - 1),
                    ) {
                        (Some(team_min), Some(team_max)) => {
                            let (team, enemy_team) = if self.team0 > self.team1 {
                                (team_max, team_min)
                            } else {
                                (team_min, team_max)
                            };
                            team_render(ui, team, 0, enemy_team);
                        }
                        _ => {}
                    }
                } else {
                    ui.centered_and_justified(|ui| ui.label("select different teams"));
                }
            });
            ui.allocate_ui_at_rect(team1_panel, |ui| {
                if self.team0 != self.team1 && self.team0.max(self.team1) < self.teams.len() {
                    let (l, r) = self.teams.split_at_mut(self.team0.min(self.team1) + 1);
                    match (
                        l.last_mut(),
                        r.get_mut(self.team0.max(self.team1) - self.team0.min(self.team1) - 1),
                    ) {
                        (Some(team_min), Some(team_max)) => {
                            let (team, enemy_team) = if self.team0 < self.team1 {
                                (team_max, team_min)
                            } else {
                                (team_min, team_max)
                            };
                            team_render(ui, team, 1, enemy_team);
                        }
                        _ => {}
                    };
                } else {
                    ui.centered_and_justified(|ui| ui.label("select different teams"));
                }
            });
            if let Some(effect) = self.damages.first_mut() {
                if !effect.render(ui) {
                    self.damages.remove(0);
                }
            }
        });
    }
}

impl DamageCalcApp {
    fn select_column(ui: &mut Ui, team: &Team, style: &Style) -> Option<usize> {
        let mut sel = None;
        ui.vertical_centered(|ui| {
            for (i, unit) in team.units.iter().enumerate() {
                let press = DamageCalcApp::select_box(
                    ui,
                    style.box_colors[i % style.box_colors.len()],
                    i == team.select,
                    unit.is_none(),
                    style,
                );
                if press {
                    sel = Some(i);
                }
            }
        });
        sel
    }
    fn select_box(ui: &mut Ui, color: Color32, selected: bool, x: bool, style: &Style) -> bool {
        let (rect, resp) = ui.allocate_exact_size(Vec2::splat(style.box_size), Sense::click());
        if selected {
            ui.painter().rect(
                rect,
                Rounding::ZERO,
                color,
                Stroke::new(style.line_size, Color32::WHITE),
            );
        } else {
            ui.painter().rect_filled(rect, Rounding::ZERO, color);
        }
        if x {
            ui.painter().line_segment(
                [rect.right_top(), rect.left_bottom()],
                Stroke::new(style.line_size, Color32::WHITE),
            );
            ui.painter().line_segment(
                [rect.right_bottom(), rect.left_top()],
                Stroke::new(style.line_size, Color32::WHITE),
            );
        }
        resp.clicked()
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
struct Team {
    name: String,
    select: usize,
    units: Vec<Option<Unit>>,
    percent: i32,
    retaliation: bool,
    second_select: usize,
}
impl Team {
    fn new(count: usize) -> Self {
        let mut units = Vec::with_capacity(count);
        for _ in 0..count {
            units.push(None);
        }
        Self {
            name: "team".to_string(),
            select: 0,
            units: units,
            percent: 100,
            retaliation: false,
            second_select: 0,
        }
    }
}
#[derive(serde::Deserialize, serde::Serialize)]
struct Style {
    fancy_stats: bool,
    box_colors: Vec<Color32>,
    box_size: f32,
    line_size: f32,
    mono: bool,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            fancy_stats: true,
            box_colors: vec![
                Color32::from_rgb(143, 209, 79),
                Color32::from_rgb(206, 231, 65),
                Color32::from_rgb(218, 0, 99),
                Color32::from_rgb(18, 205, 212),
                Color32::from_rgb(12, 167, 137),
                Color32::from_rgb(101, 44, 179),
            ],
            box_size: 60.,
            line_size: 3.,
            mono: false,
        }
    }
}

impl Style {
    fn apply_mono(&self, ctx: &egui::Context) {
        if self.mono {
            ctx.style_mut(|style| {
                for (s, font) in style.text_styles.iter_mut() {
                    if TextStyle::Monospace.eq(s) {
                        continue;
                    }
                    font.family = FontFamily::Monospace;
                }
            });
        } else {
            ctx.style_mut(|style| {
                for (s, font) in style.text_styles.iter_mut() {
                    if TextStyle::Monospace.eq(s) {
                        continue;
                    }
                    font.family = FontFamily::Proportional;
                }
            });
        }
    }
}

struct DamageEffect {
    damage: i32,
    init: bool,
    start_time: f64,
}

const ANIMATION_TIME: f64 = 5.;
impl DamageEffect {
    fn render(&mut self, ui: &mut Ui) -> bool {
        let time = ui.input(|i| i.time);
        if !self.init {
            self.init = true;
            self.start_time = time;
        }
        let p = (time - self.start_time) / ANIMATION_TIME;
        if p > 1. {
            return false;
        }
        if p > 0.5 {
            let spos = ui.max_rect().center();
            let epos = ui.max_rect().center_bottom();
            ui.ctx().request_repaint();
            let p = (p - 0.5) * 2.;
            ui.painter().text(
                spos.lerp(epos, (p * p * p) as f32),
                Align2::CENTER_CENTER,
                self.damage,
                FontId::proportional(55.),
                Color32::RED,
            );
        } else {
            let spos = ui.max_rect().center();
            ui.ctx().request_repaint();
            let disp_damage = lerp(
                0. ..=self.damage as f32,
                (p * 2.).sqrt().sqrt().sqrt() as f32,
            ) as i32;
            ui.painter().text(
                spos,
                Align2::CENTER_CENTER,
                disp_damage.to_string(),
                FontId::proportional(55.),
                Color32::RED,
            );
        }
        true
    }
    fn new(dmg: i32) -> Self {
        Self {
            damage: dmg,
            init: false,
            start_time: 0.,
        }
    }
}
