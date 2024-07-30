use egui::{Color32, Rounding, Sense, Stroke, Ui, Vec2};
/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct DamageCalcApp {
    select1: i32,
    select2: i32,
}

impl Default for DamageCalcApp {
    fn default() -> Self {
        Self {
            select1: 0,
            select2: 0,
        }
    }
}

impl DamageCalcApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
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
        egui::CentralPanel::default().show(ctx, |ui| {
            //ui.heading("DamageCalc");

            let full_screen = ui.max_rect();
            let (select_colum1, left_screen) =
                full_screen.split_left_right_at_x(BOX_SIZE + ui.style().spacing.item_spacing.x);
            let (left_screen, select_colum2) = left_screen.split_left_right_at_x(
                full_screen.width() - BOX_SIZE + ui.style().spacing.item_spacing.x,
            );
            ui.allocate_ui_at_rect(select_colum1, |ui| {
                if let Some(sel) = self.select_column(ui, self.select1) {
                    self.select1 = sel
                }
            });
            ui.allocate_ui_at_rect(select_colum2, |ui| {
                if let Some(sel) = self.select_column(ui, self.select2) {
                    self.select2 = sel
                }
            });
        });
    }
}

impl DamageCalcApp {
    fn select_column(&mut self, ui: &mut Ui, cur_sel: i32) -> Option<i32> {
        let mut sel: Option<i32> = None;
        for i in 0..6 {
            let press = DamageCalcApp::select_box(ui, COLORS[i], i as i32 == cur_sel, true);
            if press {
                sel = Some(i as i32);
            }
            //ui.add_space(10f32);
        }
        sel
    }
    fn select_box(ui: &mut Ui, color: Color32, selected: bool, x: bool) -> bool {
        let (rect, resp) = ui.allocate_exact_size(Vec2::splat(BOX_SIZE), Sense::click());
        if selected {
            ui.painter().rect(
                rect,
                Rounding::ZERO,
                color,
                Stroke::new(LINE_SIZE, Color32::WHITE),
            );
        } else {
            ui.painter().rect_filled(rect, Rounding::ZERO, color);
        }
        if x {
            ui.painter().line_segment(
                [rect.right_top(), rect.left_bottom()],
                Stroke::new(LINE_SIZE, Color32::WHITE),
            );
            ui.painter().line_segment(
                [rect.right_bottom(), rect.left_top()],
                Stroke::new(LINE_SIZE, Color32::WHITE),
            );
        }
        resp.clicked()
    }
}

const BOX_SIZE: f32 = 60.;
const LINE_SIZE: f32 = 5.;
const COLORS: [Color32; 6] = [
    Color32::from_rgb(143, 209, 79),
    Color32::from_rgb(206, 231, 65),
    Color32::from_rgb(218, 0, 99),
    Color32::from_rgb(18, 205, 212),
    Color32::from_rgb(12, 167, 137),
    Color32::from_rgb(101, 44, 179),
];
