//! State aplikasi + navigasi (pengganti `App.tsx` + `useGameStore` v2).
//!
//! Satu arah: UI mengirim `dungeon_core::Action` lewat [`App::apply`];
//! `dungeon-core` menghitung state baru + `Effect`; aplikasi menampilkan
//! efek (mis. modal naik level) dan menyimpan state (penyimpanan = Fase 4,
//! jadi untuk Fase 3 state hanya hidup di memori selama aplikasi jalan).
//!
//! State UI transien v2 (`currentView`, `selectedTopicId`, modal, dll.)
//! tinggal di sini, bukan di core (sesuai keputusan desain Fase 2).

use std::time::{SystemTime, UNIX_EPOCH};

use dungeon_core::{
    dates::{civil_from_days, format_date_key},
    formulas::{get_display_streak, get_level_progress},
    lesson_engine::get_due_review_items,
    reducer::{Action, ActionContext, Effect, Reducer},
    state::GameState,
};
use egui::{Align, Color32, Layout, Ui};

use crate::ui::{
    dialogs::{battle::BattleUi, boss::BossUi, lesson::LessonUi},
    screens::{
        camp::CampUi, grimoire::GrimoireUi, review::ReviewUi, settings::SettingsUi,
        tower::TowerUi,
    },
    theme::{self, color},
};

/// Layar aplikasi (9 view v2 — SDD Bagian 9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Title,
    Dungeon,
    Grimoire,
    Review,
    Camp,
    Dashboard,
    Inventory,
    Tower,
    Settings,
}

/// Dialog yang sedang terbuka (di atas layar aktif).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Dialog {
    /// Pertarungan ruangan (`BattleModal` v2).
    Battle,
    /// Pertarungan boss (`BossModal` v2).
    Boss,
    /// Penampil scroll materi (`StandaloneScrollModal` v2).
    Scroll { topic_id: String },
    /// Pemain chunk pelajaran (`LessonChunkPlayer` v2).
    Lesson,
    /// Pemberitahuan generik (mis. fitur fase berikutnya).
    Notice { title: String, body: String },
}

/// Hadiah ruangan terakhir (untuk layar hasil battle).
#[derive(Debug, Clone, Default)]
pub struct RoomReward {
    pub total_xp: i64,
    pub base_xp: i64,
    pub bonus_xp: i64,
    pub echo_xp: i64,
    pub unlocked_spell_id: Option<String>,
}

/// State aplikasi.
pub struct App {
    pub state: GameState,
    pub screen: Screen,
    pub dialog: Option<Dialog>,
    /// Modal naik level yang sedang ditampilkan (dari `Effect::LevelUp`).
    pub level_up: Option<(i64, i64, String)>,
    /// Hadiah battle ruangan terakhir.
    pub room_reward: Option<RoomReward>,
    /// Hadiah boss terakhir (XP).
    pub boss_reward: Option<i64>,
    /// Input nama + avatar di Title.
    pub title_name: String,
    pub title_avatar: String,
    /// State dialog battle/boss/lesson (dibuat saat dialog dibuka).
    pub battle: Option<BattleUi>,
    pub boss: Option<BossUi>,
    pub lesson: Option<LessonUi>,
    /// State mini-lab scroll (slider neuron/softmax/quant/latency).
    pub scroll: crate::ui::dialogs::scroll::ScrollUi,
    /// State per layar.
    pub grimoire: GrimoireUi,
    pub review: ReviewUi,
    pub camp: CampUi,
    pub tower: TowerUi,
    pub settings: SettingsUi,
    /// Pesan singkat di bawah layar (toast).
    pub toast: Option<String>,
    /// Id layer stabil untuk overlay CRT (dibuat sekali).
    pub crt_layer: egui::Id,
    /// RNG untuk pengacakan deck/latihan (diseed dari waktu).
    pub rng: dungeon_core::rng::XorShift64,
}

impl App {
    pub fn new() -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E3779B97F4A7C15);
        Self {
            state: GameState::default(),
            screen: Screen::Title,
            dialog: None,
            level_up: None,
            room_reward: None,
            boss_reward: None,
            title_name: "Petualang Tensor".to_string(),
            title_avatar: "alchemist".to_string(),
            battle: None,
            boss: None,
            lesson: None,
            scroll: crate::ui::dialogs::scroll::ScrollUi::default(),
            grimoire: GrimoireUi::default(),
            review: ReviewUi::default(),
            camp: CampUi::default(),
            tower: TowerUi::default(),
            settings: SettingsUi::default(),
            toast: None,
            crt_layer: egui::Id::unique("crt_overlay"),
            rng: dungeon_core::rng::XorShift64::new(seed),
        }
    }

    /// Konteks aksi: tanggal lokal hari ini + stempel ISO sekarang.
    pub fn ctx(&self) -> ActionContext {
        ActionContext {
            today_key: today_key(),
            now_iso: now_iso(),
        }
    }

    /// Kirim aksi ke reducer; tangani efeknya untuk UI.
    pub fn apply(&mut self, action: Action) {
        let ctx = self.ctx();
        let old = std::mem::take(&mut self.state);
        let (new_state, effects) = Reducer::apply(old, action, &ctx);
        self.state = new_state;
        for effect in effects {
            self.handle_effect(effect);
        }
    }

    fn handle_effect(&mut self, effect: Effect) {
        match effect {
            Effect::LevelUp {
                old_level,
                new_level,
                new_title,
            } => {
                self.level_up = Some((old_level, new_level, new_title));
            }
            Effect::RoomDefeated {
                xp_gained,
                base_xp_gained,
                bonus_xp_gained,
                echo_xp_gained,
                unlocked_spell_id,
                ..
            } => {
                self.room_reward = Some(RoomReward {
                    total_xp: xp_gained,
                    base_xp: base_xp_gained,
                    bonus_xp: bonus_xp_gained,
                    echo_xp: echo_xp_gained,
                    unlocked_spell_id,
                });
            }
            Effect::BossDefeated { xp_gained, .. } => {
                self.boss_reward = Some(xp_gained);
            }
            _ => {}
        }
    }

    /// Buka battle ruangan (dari peta dungeon).
    pub fn open_battle(&mut self, topic_id: &str) {
        self.battle = Some(BattleUi::open(self, topic_id));
        self.dialog = Some(Dialog::Battle);
    }

    /// Buka battle boss lantai.
    pub fn open_boss(&mut self, floor_id: u32) {
        self.boss = Some(BossUi::open(self, floor_id));
        self.dialog = Some(Dialog::Boss);
    }

    /// Buka pemain chunk pelajaran.
    pub fn open_lesson(&mut self, chunk_id: &str) {
        self.lesson = Some(LessonUi::open(chunk_id));
        self.dialog = Some(Dialog::Lesson);
    }

    /// Tutup dialog yang sedang terbuka.
    pub fn close_dialog(&mut self) {
        self.dialog = None;
        self.battle = None;
        self.boss = None;
        self.lesson = None;
    }

    /// Jumlah item review yang jatuh tempo hari ini (badge nav).
    pub fn due_review_count(&self) -> usize {
        get_due_review_items(&self.state.review_queue, &today_key(), 10).total_due_count
    }

    /// Gambar seluruh UI untuk satu frame (egui 0.37: panel & konten
    /// digambar di dalam `Ui` root dari `run_ui`).
    pub fn show(&mut self, ui: &mut Ui) {
        if self.screen == Screen::Title {
            egui::CentralPanel::default()
                .frame(egui::Frame::default().fill(color::ABYSS))
                .show(ui, |ui| {
                    crate::ui::screens::title::show_title(self, ui);
                });
        } else {
            self.show_xp_bar(ui);
            egui::CentralPanel::default()
                .frame(egui::Frame::default().fill(color::ABYSS))
                .show(ui, |ui| {
                    let avail = ui.available_width();
                    let max_w = theme::CONTENT_MAX_WIDTH.min(avail);
                    ui.vertical_centered(|ui| {
                        ui.set_max_width(max_w);
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                ui.add_space(theme::SAFE_TOP);
                                match self.screen {
                                    Screen::Dungeon => {
                                        crate::ui::screens::dungeon::show_dungeon(self, ui)
                                    }
                                    Screen::Grimoire => {
                                        crate::ui::screens::grimoire::show_grimoire(self, ui)
                                    }
                                    Screen::Review => {
                                        crate::ui::screens::review::show_review(self, ui)
                                    }
                                    Screen::Camp => {
                                        crate::ui::screens::camp::show_camp(self, ui)
                                    }
                                    Screen::Dashboard => {
                                        crate::ui::screens::dashboard::show_dashboard(self, ui)
                                    }
                                    Screen::Inventory => {
                                        crate::ui::screens::inventory::show_inventory(self, ui)
                                    }
                                    Screen::Tower => {
                                        crate::ui::screens::tower::show_tower(self, ui)
                                    }
                                    Screen::Settings => {
                                        crate::ui::screens::settings::show_settings(self, ui)
                                    }
                                    Screen::Title => unreachable!(),
                                }
                                ui.add_space(theme::SAFE_BOTTOM + 8.0);
                                if let Some(toast) = &self.toast {
                                    ui.label(theme::vt_text(toast, 20.0, color::GOLD));
                                }
                            });
                    });
                });
        }

        // Dialog di atas layar (modal v2 menjadi dialog layar-penuh-aman).
        self.show_dialog(ui);
        self.show_level_up(ui);
        // Overlay scanline CRT halus (nonaktif saat reduce-motion).
        if !self.state.reduce_motion {
            self.paint_crt(ui);
        }
    }

    /// Bilah XP + strip navigasi (pengganti `XPBar.tsx` v2).
    fn show_xp_bar(&mut self, ui: &mut Ui) {
        let level = get_level_progress(self.state.xp);
        let streak = get_display_streak(self.state.streak, self.state.last_active_date.as_deref(), &today_key());
        let due = self.due_review_count();
        let tower_unlocked =
            dungeon_core::formulas::is_floor_unlocked(6, &self.state.defeated_bosses);
        let screen = self.screen;

        egui::Panel::top("xp_bar")
            .frame(egui::Frame::default().fill(color::ABYSS))
            .show(ui, |ui| {
                ui.add_space(4.0);
                theme::pixel_box(ui, theme::PixelBox::Normal, |ui| {
                    // Baris 1: avatar + nama/level + XP bar.
                    ui.horizontal(|ui| {
                        let (rect, _) = ui.allocate_exact_size(
                            egui::vec2(56.0, 56.0),
                            egui::Sense::hover(),
                        );
                        ui.painter().rect_filled(rect, 0.0, color::PANEL_DARK);
                        ui.painter().rect_stroke(
                            rect,
                            0.0,
                            egui::Stroke::new(2.0, color::GOLD),
                            egui::StrokeKind::Outside,
                        );
                        crate::ui::sprites::draw_avatar(
                            ui.painter(),
                            rect.shrink(6.0),
                            &self.state.avatar_id,
                        );
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(theme::pixel_text(&self.state.player_name, 13.0, color::CREAM));
                                ui.label(theme::pixel_text(
                                    &format!("Lv.{}", level.level),
                                    10.0,
                                    color::GOLD,
                                ));
                            });
                            ui.label(theme::vt_text(
                                &format!("Gelar: {} · Streak: {} hari", level.title, streak),
                                19.0,
                                color::GREEN,
                            ));
                        });
                    });
                    ui.add_space(6.0);
                    theme::xp_bar(
                        ui,
                        level.progress_percent as f32 / 100.0,
                        &format!(
                            "{} / {} XP ({}%)",
                            level.xp_in_current_level,
                            level.xp_needed_for_next_level,
                            level.progress_percent
                        ),
                    );
                    ui.label(theme::vt_text(
                        &format!(
                            "EXP PETUALANG · {} XP (Lv.{}: {} XP)",
                            self.state.xp,
                            level.level + 1,
                            level.next_level_xp
                        ),
                        18.0,
                        color::MUTED,
                    ));
                    // Baris 2: tombol cepat.
                    ui.horizontal_wrapped(|ui| {
                        if theme::pixel_button(ui, "✦ Portal Dimensi 3D", color::DUNGEON, color::CYAN)
                            .clicked()
                        {
                            self.dialog = Some(Dialog::Notice {
                                title: "Portal Dimensi 3D".to_string(),
                                body: "Dungeon 3D (WebView khusus, Three.js v2) hadir di Milestone 5. Untuk sekarang, peta 2D di bawah ini sudah bisa dimainkan.".to_string(),
                            });
                        }
                        if theme::pixel_button(ui, "Aula Guild (F5)", color::RED_BOX_BG, color::GOLD)
                            .clicked()
                        {
                            self.apply(Action::SelectFloor { floor_id: 5 });
                            self.screen = Screen::Dungeon;
                        }
                        let sound_label = if self.state.sound_enabled {
                            "Suara: ON"
                        } else {
                            "Suara: OFF"
                        };
                        let sound_bg = if self.state.sound_enabled {
                            color::GREEN_DARK
                        } else {
                            color::DUNGEON
                        };
                        if theme::pixel_button(ui, sound_label, sound_bg, color::CREAM).clicked() {
                            self.apply(Action::ToggleSound);
                        }
                    });
                    ui.add_space(4.0);
                    // Strip navigasi 8 item (v2: XPBar navItems).
                    ui.horizontal_wrapped(|ui| {
                        let items: [(Screen, &str, Option<String>); 8] = [
                            (
                                Screen::Dungeon,
                                "Peta Dungeon",
                                Some(format!("L{}", self.state.active_floor_id.min(5).max(1))),
                            ),
                            (
                                Screen::Grimoire,
                                "Grimoire & Dojo",
                                Some(format!("{}/28", self.state.spells_unlocked.len())),
                            ),
                            (Screen::Review, "Ulasan Harian", Some(format!("{due}"))),
                            (Screen::Camp, "Camp Api Unggun", None),
                            (
                                Screen::Tower,
                                "Menara Master",
                                Some(if tower_unlocked {
                                    "TERBUKA".to_string()
                                } else {
                                    "L6".to_string()
                                }),
                            ),
                            (
                                Screen::Dashboard,
                                "Karakter & Statistik",
                                Some(format!("{}/28", self.state.completed_rooms.len())),
                            ),
                            (Screen::Inventory, "Inventori Relik", None),
                            (Screen::Settings, "Pengaturan", None),
                        ];
                        for (target, label, badge) in items {
                            let full = match badge {
                                Some(b) => format!("{label} {b}"),
                                None => label.to_string(),
                            };
                            if theme::pixel_chip(ui, &full, screen == target).clicked() {
                                self.screen = target;
                            }
                        }
                    });
                });
            });
    }

    /// Dialog modal yang sedang terbuka.
    fn show_dialog(&mut self, ui: &mut Ui) {
        let Some(dialog) = self.dialog.clone() else {
            return;
        };
        let ctx = ui.ctx();
        match dialog {
            Dialog::Battle => {
                if let Some(mut battle) = self.battle.take() {
                    let open = crate::ui::dialogs::battle::show_battle(self, ctx, &mut battle);
                    if open {
                        self.battle = Some(battle);
                    } else {
                        self.dialog = None;
                    }
                }
            }
            Dialog::Boss => {
                if let Some(mut boss) = self.boss.take() {
                    let open = crate::ui::dialogs::boss::show_boss(self, ctx, &mut boss);
                    if open {
                        self.boss = Some(boss);
                    } else {
                        self.dialog = None;
                    }
                }
            }
            Dialog::Scroll { topic_id } => {
                let open =
                    crate::ui::dialogs::scroll::show_scroll(self, ctx, &topic_id);
                if !open {
                    self.dialog = None;
                }
            }
            Dialog::Lesson => {
                if let Some(mut lesson) = self.lesson.take() {
                    let open = crate::ui::dialogs::lesson::show_lesson(self, ctx, &mut lesson);
                    if open {
                        self.lesson = Some(lesson);
                    } else {
                        self.dialog = None;
                    }
                }
            }
            Dialog::Notice { title, body } => {
                let mut open = true;
                let mut close = false;
                egui::Window::new(theme::pixel_text(&title, 14.0, color::GOLD))
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .collapsible(false)
                    .resizable(false)
                    .open(&mut open)
                    .show(ctx, |ui| {
                        ui.set_max_width(340.0);
                        ui.label(theme::vt_text(&body, 21.0, color::CREAM));
                        ui.add_space(8.0);
                        if theme::pixel_button(ui, "Tutup", color::GREEN_DARK, color::CREAM)
                            .clicked()
                        {
                            close = true;
                        }
                    });
                if close || !open {
                    self.dialog = None;
                }
            }
        }
    }

    /// Modal naik level (dari `Effect::LevelUp`).
    fn show_level_up(&mut self, ui: &mut Ui) {
        let Some((old_level, new_level, title)) = self.level_up.clone() else {
            return;
        };
        let ctx = ui.ctx();
        let mut open = true;
        let mut close = false;
        egui::Window::new(theme::pixel_text("NAIK LEVEL!", 14.0, color::GOLD))
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.set_max_width(320.0);
                ui.vertical_centered(|ui| {
                    ui.label(theme::pixel_text(
                        &format!("Lv.{old_level} → Lv.{new_level}"),
                        16.0,
                        color::GREEN,
                    ));
                    ui.label(theme::vt_text(
                        &format!("Gelar baru: {title}"),
                        24.0,
                        color::GOLD,
                    ));
                    ui.add_space(8.0);
                    if theme::pixel_button(ui, "Lanjutkan Petualangan", color::GREEN_DARK, color::CREAM)
                        .clicked()
                    {
                        close = true;
                    }
                });
            });
        if close || !open {
            self.level_up = None;
        }
    }

    /// Overlay scanline CRT halus + vignette (pixel.css `.crt-overlay`).
    /// Garis hitam tipis tiap 4px + gelap di tepi; sangat transparan.
    fn paint_crt(&self, ui: &mut Ui) {
        let ctx = ui.ctx();
        let screen = ctx.viewport_rect();
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            self.crt_layer,
        ));
        let line = Color32::from_black_alpha(14);
        let mut y = screen.top();
        while y < screen.bottom() {
            painter.rect_filled(
                egui::Rect::from_min_size(egui::pos2(screen.left(), y + 2.0), egui::vec2(screen.width(), 2.0)),
                0.0,
                line,
            );
            y += 4.0;
        }
        let edge = Color32::from_black_alpha(28);
        painter.rect_filled(
            egui::Rect::from_min_size(screen.left_top(), egui::vec2(screen.width(), 10.0)),
            0.0,
            edge,
        );
        painter.rect_filled(
            egui::Rect::from_min_size(
                egui::pos2(screen.left(), screen.bottom() - 10.0),
                egui::vec2(screen.width(), 10.0),
            ),
            0.0,
            edge,
        );
    }
}

/// Kunci tanggal lokal hari ini (`YYYY-MM-DD`), memakai zona waktu
/// perangkat via `localtime_r` (bionic Android). Core tidak pernah
/// membaca jam; aplikasi yang memasoknya (keputusan desain Fase 2).
pub fn today_key() -> String {
    let (y, m, d) = local_ymd();
    format_date_key(y, m, d)
}

/// Stempel ISO 8601 UTC sekarang (setara `new Date().toISOString()` v2).
pub fn now_iso() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let (y, m, d) = civil_from_days(secs.div_euclid(86_400));
    let rem = secs.rem_euclid(86_400);
    let (hh, mm, ss) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}Z")
}

/// Tanggal lokal perangkat (tahun, bulan, hari) via `libc::localtime_r`.
fn local_ymd() -> (i32, u32, u32) {
    unsafe {
        let t = libc::time(std::ptr::null_mut());
        let mut tm: libc::tm = std::mem::zeroed();
        if libc::localtime_r(&t, &mut tm).is_null() {
            // Fallback: UTC bila localtime gagal.
            let secs = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            return civil_from_days(secs.div_euclid(86_400));
        }
        (
            tm.tm_year + 1900,
            (tm.tm_mon + 1) as u32,
            tm.tm_mday as u32,
        )
    }
}

/// Bantuan: label `Lv.{level} ({title})`.
pub fn level_label(level: i64, title: &str) -> String {
    format!("Lv.{level} ({title})")
}

/// Bantuan: susun `Ui` horizontal dengan label kiri + konten kanan.
pub fn labeled_row(ui: &mut Ui, label: &str, add: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.label(theme::pixel_text(label, 10.0, color::GOLD));
        ui.with_layout(Layout::left_to_right(Align::Center), add);
    });
}
