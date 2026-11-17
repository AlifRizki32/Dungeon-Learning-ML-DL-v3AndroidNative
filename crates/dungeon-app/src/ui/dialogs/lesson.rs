//! Pemain chunk pelajaran (`LessonChunkPlayer.tsx` v2, 2289 baris).
//!
//! Halaman berurutan: intro → istilah → adegan → tebakan → visual →
//! langkah → rumus → latihan (choice/fillNumber/order/match) →
//! kesalahan umum → tugas nyata + ringkasan → selesai. Chunk selesai
//! bila semua aktivitas latihan pernah dicoba (`is_chunk_complete`);
//! penyelesaian menjadwalkan kartu review (`CompleteLessonChunk`).
//!
//! Catatan paritas: pilihan jawaban latihan ditampilkan dalam urutan
//! asli konten (v2 mengacaknya); pengacakan dicatat di ui.md.

use std::collections::BTreeMap;

use dungeon_core::{
    lesson_engine::{
        evaluate_fill_number, evaluate_match_attempt, evaluate_order_attempt,
        get_lesson_chunk_by_id, is_chunk_complete,
    },
    reducer::Action,
};
use dungeon_data::lessons::{LessonPracticeActivity, LessonVisual};
use egui::{Context, Ui};

use crate::{
    app::App,
    ui::theme::{self, color, PixelBox},
};

#[derive(Clone)]
enum PracticeState {
    Choice {
        selected: Option<usize>,
    },
    Fill {
        input: String,
        result: Option<bool>,
    },
    Order {
        picked: Vec<String>,
        result: Option<bool>,
    },
    Match {
        picks: BTreeMap<usize, String>,
        result: Option<bool>,
    },
}

pub struct LessonUi {
    pub chunk_id: String,
    pub page: usize,
    pub guess_selected: Option<usize>,
    practice: BTreeMap<String, PracticeState>,
    pub summary_text: String,
    pub completed: bool,
}

impl LessonUi {
    pub fn open(chunk_id: &str) -> Self {
        Self {
            chunk_id: chunk_id.to_string(),
            page: 0,
            guess_selected: None,
            practice: BTreeMap::new(),
            summary_text: String::new(),
            completed: false,
        }
    }

    /// Jumlah halaman untuk chunk ini (visual & tugas nyata opsional).
    fn page_count(chunk: &dungeon_data::lessons::LessonChunk) -> usize {
        // intro, istilah, adegan, tebakan, [visual], langkah, rumus,
        // latihan, kesalahan umum, [tugas nyata + ringkasan], selesai
        let mut n = 9;
        if chunk.visual.is_some() {
            n += 1;
        }
        if chunk.real_task.is_some() {
            n += 1;
        }
        n
    }

    fn practice_state(&mut self, id: &str, activity: &LessonPracticeActivity) -> &mut PracticeState {
        self.practice.entry(id.to_string()).or_insert_with(|| match activity {
            LessonPracticeActivity::Choice { .. } => PracticeState::Choice { selected: None },
            LessonPracticeActivity::FillNumber { .. } => PracticeState::Fill {
                input: String::new(),
                result: None,
            },
            LessonPracticeActivity::Order { .. } => PracticeState::Order {
                picked: Vec::new(),
                result: None,
            },
            LessonPracticeActivity::Match { .. } => PracticeState::Match {
                picks: BTreeMap::new(),
                result: None,
            },
            LessonPracticeActivity::Unknown => PracticeState::Choice { selected: None },
        })
    }
}

/// Gambar dialog pelajaran. Mengembalikan `true` bila masih terbuka.
pub fn show_lesson(app: &mut App, ctx: &Context, lesson: &mut LessonUi) -> bool {
    let mut open = true;
    let chunk = get_lesson_chunk_by_id(&lesson.chunk_id);
    let title = chunk
        .map(|c| c.title.clone())
        .unwrap_or_else(|| "Pelajaran".to_string());
    let mut close_after = false;

    egui::Window::new(theme::pixel_text(&title, 12.0, color::CREAM))
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .collapsible(false)
        .resizable(false)
        .open(&mut open)
        .show(ctx, |ui| {
            ui.set_max_width(380.0);
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .max_height(ctx.viewport_rect().height() * 0.7)
                .show(ui, |ui| {
                    show_lesson_body(app, ui, lesson);
                });
            ui.add_space(6.0);
            show_lesson_nav(app, ui, lesson, &mut close_after);
        });

    if close_after {
        open = false;
    }
    open
}

fn show_lesson_body(app: &mut App, ui: &mut Ui, lesson: &mut LessonUi) {
    let Some(chunk) = get_lesson_chunk_by_id(&lesson.chunk_id) else {
        ui.label(theme::vt_text("Chunk pelajaran tidak ditemukan.", 21.0, color::RED));
        return;
    };
    // Indeks halaman logis (visual & tugas nyata menggeser halaman).
    let has_visual = chunk.visual.is_some();
    let has_real = chunk.real_task.is_some();
    let page = lesson.page;
    // Peta halaman: 0 intro, 1 istilah, 2 adegan, 3 tebakan,
    // 4 visual?, lalu langkah, rumus, latihan, kesalahan, [tugas], selesai.
    match page {
        0 => {
            ui.label(theme::pixel_text("TUJUAN", 11.0, color::GOLD));
            ui.label(theme::vt_text(&chunk.goal, 22.0, color::CREAM));
            if let Some(ml) = &chunk.mini_lab {
                ui.label(theme::vt_text(
                    &format!("Mini-lab terkait: {ml} (buka lewat scroll topik)."),
                    19.0,
                    color::MUTED,
                ));
            }
        }
        1 => {
            ui.label(theme::pixel_text("ISTILAH KUNCI", 11.0, color::GOLD));
            for term in &chunk.terms {
                ui.label(theme::vt_text(&format!("{} — {}", term.term, term.plain), 21.0, color::CREAM));
            }
        }
        2 => {
            ui.label(theme::pixel_text("ADEGAN", 11.0, color::GOLD));
            for line in &chunk.scene {
                ui.label(theme::vt_text(
                    &format!("{}: {}", line.who, line.text),
                    21.0,
                    color::CREAM,
                ));
            }
        }
        3 => show_guess(app, ui, lesson),
        _ => {
            // Halaman setelah tebakan (indeks bergeser bila ada visual).
            let mut p = page - 4;
            if has_visual {
                if p == 0 {
                    if let Some(visual) = &chunk.visual {
                        show_visual(ui, visual);
                    }
                    return;
                }
                p -= 1;
            }
            match p {
                0 => {
                    ui.label(theme::pixel_text("LANGKAH", 11.0, color::GOLD));
                    for (i, step) in chunk.steps.iter().enumerate() {
                        ui.label(theme::vt_text(
                            &format!("{}. {}", i + 1, step.text),
                            21.0,
                            color::CREAM,
                        ));
                        for h in &step.highlight {
                            ui.label(theme::vt_text(&format!("  ▸ {h}"), 19.0, color::GOLD));
                        }
                    }
                }
                1 => {
                    ui.label(theme::pixel_text("RUMUS", 11.0, color::GOLD));
                    if let Some(ft) = &chunk.formula_title {
                        ui.label(theme::vt_text(ft, 20.0, color::GOLD));
                    }
                    for line in &chunk.formula.lines {
                        ui.label(theme::vt_text(line, 21.0, color::CREAM));
                    }
                    ui.label(theme::vt_text(
                        &format!("Dibaca: {}", chunk.formula.read_aloud),
                        19.0,
                        color::MUTED,
                    ));
                }
                2 => show_practice(app, ui, lesson),
                3 => {
                    ui.label(theme::pixel_text("KESALAHAN UMUM", 11.0, color::RED));
                    ui.label(theme::vt_text(&chunk.common_mistake, 21.0, color::CREAM));
                }
                _ => {
                    let mut q = p - 4;
                    if has_real {
                        if q == 0 {
                            show_real_task(app, ui, lesson);
                            return;
                        }
                        q -= 1;
                    }
                    if q == 0 {
                        show_finish(app, ui, lesson);
                    }
                }
            }
        }
    }
}

fn show_guess(app: &mut App, ui: &mut Ui, lesson: &mut LessonUi) {
    let Some(chunk) = get_lesson_chunk_by_id(&lesson.chunk_id) else {
        return;
    };
    let _ = app;
    ui.label(theme::pixel_text("TEBAKAN PEMBUKA", 11.0, color::GOLD));
    ui.label(theme::vt_text(&chunk.guess.question, 22.0, color::CREAM));
    ui.add_space(4.0);
    let selected = lesson.guess_selected;
    for (i, opt) in chunk.guess.options.iter().enumerate() {
        let chosen = selected == Some(i);
        let bg = if chosen {
            if i == chunk.guess.correct_index {
                color::GREEN_DARK
            } else {
                color::RED_DARK
            }
        } else {
            color::DUNGEON
        };
        let label = format!("{}. {}", (b'A' + i as u8) as char, opt);
        if theme::pixel_button(ui, &label, bg, color::CREAM).clicked() && selected.is_none() {
            lesson.guess_selected = Some(i);
        }
    }
    if let Some(i) = selected {
        ui.add_space(4.0);
        ui.label(theme::vt_text(
            if i == chunk.guess.correct_index {
                "✓ Benar!"
            } else {
                "✗ Belum tepat."
            },
            21.0,
            if i == chunk.guess.correct_index {
                color::GREEN
            } else {
                color::RED
            },
        ));
        ui.label(theme::vt_text(&chunk.guess.reveal, 20.0, color::GOLD));
    }
}

fn show_visual(ui: &mut Ui, visual: &LessonVisual) {
    ui.label(theme::pixel_text("VISUAL", 11.0, color::GOLD));
    match visual {
        LessonVisual::DotGrid { total, columns, groups } => {
            ui.label(theme::vt_text(
                &format!("Total {total} titik, {columns} kolom:"),
                20.0,
                color::CREAM,
            ));
            for g in groups {
                ui.label(theme::vt_text(
                    &format!("• {}: {} titik", g.label, g.count),
                    20.0,
                    color::GOLD,
                ));
            }
        }
        LessonVisual::Bars { items } => {
            let max = items.iter().map(|i| i.value).fold(0.0f64, f64::max).max(1.0);
            for item in items {
                ui.label(theme::vt_text(
                    &format!("{}: {:.2}", item.label, item.value),
                    20.0,
                    color::CREAM,
                ));
                theme::xp_bar(ui, (item.value / max) as f32, "");
            }
        }
        LessonVisual::LinePlot { points, line } => {
            if let Some(l) = line {
                ui.label(theme::vt_text(
                    &format!("Garis: y = {:.2}x + {:.2}", l.m, l.c),
                    20.0,
                    color::GOLD,
                ));
            }
            for (x, y) in points {
                ui.label(theme::vt_text(&format!("({x:.2}, {y:.2})"), 19.0, color::CREAM));
            }
        }
        LessonVisual::MatrixGrid { matrices } => {
            for m in matrices {
                ui.label(theme::vt_text(&format!("Matriks {}", m.id), 20.0, color::GOLD));
                for (ri, row) in m.rows.iter().enumerate() {
                    let label = m
                        .row_labels
                        .as_ref()
                        .and_then(|l| l.get(ri))
                        .map(|s| format!("{s}: "))
                        .unwrap_or_default();
                    let cells: Vec<String> =
                        row.iter().map(|v| format!("{v:.2}")).collect();
                    ui.label(theme::vt_text(&format!("{label}[{}]", cells.join(", ")), 19.0, color::CREAM));
                }
            }
        }
        LessonVisual::CodeBlock { lines } => {
            theme::pixel_box(ui, PixelBox::Normal, |ui| {
                for line in lines {
                    ui.label(theme::vt_text(line, 19.0, color::CYAN));
                }
            });
        }
        LessonVisual::Unknown => {
            ui.label(theme::vt_text("Visual tidak dikenal.", 20.0, color::MUTED));
        }
    }
}

fn show_practice(app: &mut App, ui: &mut Ui, lesson: &mut LessonUi) {
    let Some(chunk) = get_lesson_chunk_by_id(&lesson.chunk_id) else {
        return;
    };
    ui.label(theme::pixel_text("LATIHAN", 11.0, color::GOLD));
    ui.label(theme::vt_text(
        "Coba semua latihan — chunk selesai bila semuanya pernah dicoba.",
        19.0,
        color::MUTED,
    ));
    let all_ids: Vec<String> = chunk
        .practice
        .iter()
        .map(|a| a.id().to_string())
        .collect();
    for activity in chunk.practice.iter() {
        ui.add_space(6.0);
        match activity {
            LessonPracticeActivity::Choice { id, question, options, correct_index, why } => {
                ui.label(theme::vt_text(question, 21.0, color::CREAM));
                let mut state = lesson
                    .practice_state(id, activity)
                    .clone();
                let selected = match &state {
                    PracticeState::Choice { selected } => *selected,
                    _ => None,
                };
                for (i, opt) in options.iter().enumerate() {
                    let chosen = selected == Some(i);
                    let bg = if chosen {
                        if i == *correct_index {
                            color::GREEN_DARK
                        } else {
                            color::RED_DARK
                        }
                    } else {
                        color::DUNGEON
                    };
                    let label = format!("{}. {}", (b'A' + i as u8) as char, opt);
                    if theme::pixel_button(ui, &label, bg, color::CREAM).clicked()
                        && selected.is_none()
                    {
                        record_attempt(app, &lesson.chunk_id, id, &all_ids);
                        state = PracticeState::Choice { selected: Some(i) };
                        lesson.practice.insert(id.clone(), state.clone());
                    }
                }
                if let Some(i) = selected {
                    ui.label(theme::vt_text(
                        if i == *correct_index { "✓ Benar!" } else { "✗ Salah." },
                        20.0,
                        if i == *correct_index { color::GREEN } else { color::RED },
                    ));
                    ui.label(theme::vt_text(why, 19.0, color::MUTED));
                }
            }
            LessonPracticeActivity::FillNumber { id, question, answer, tolerance, unit, why } => {
                ui.label(theme::vt_text(question, 21.0, color::CREAM));
                let mut state = lesson.practice_state(id, activity).clone();
                let (mut input, mut result) = match &state {
                    PracticeState::Fill { input, result } => (input.clone(), *result),
                    _ => (String::new(), None),
                };
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut input)
                            .font(egui::FontId::new(
                                20.0,
                                egui::FontFamily::Name(theme::FONT_VT.into()),
                            ))
                            .desired_width(120.0)
                            .hint_text("Jawaban angka"),
                    );
                    ui.label(theme::vt_text(unit, 20.0, color::MUTED));
                    if theme::pixel_button(ui, "Periksa", color::GREEN_DARK, color::CREAM)
                        .clicked()
                    {
                        let eval = evaluate_fill_number(&input, *answer, *tolerance);
                        record_attempt(app, &lesson.chunk_id, id, &all_ids);
                        result = Some(eval.is_correct);
                        state = PracticeState::Fill { input: input.clone(), result };
                        lesson.practice.insert(id.clone(), state.clone());
                    }
                });
                // Simpan input yang sedang diketik tanpa menandai selesai.
                if let PracticeState::Fill { input: cur, .. } = lesson.practice_state(id, activity) {
                    if *cur != input {
                        *cur = input.clone();
                    }
                }
                if let Some(ok) = result {
                    let msg = if ok {
                        format!("✓ Benar! Jawaban: {answer} {unit}")
                    } else {
                        format!("✗ Belum tepat. Jawaban: {answer} {unit}")
                    };
                    ui.label(theme::vt_text(
                        &msg,
                        20.0,
                        if ok { color::GREEN } else { color::RED },
                    ));
                    ui.label(theme::vt_text(why, 19.0, color::MUTED));
                }
            }
            LessonPracticeActivity::Order { id, question, items, why } => {
                ui.label(theme::vt_text(question, 21.0, color::CREAM));
                ui.label(theme::vt_text(
                    "Ketuk item sesuai urutan yang benar:",
                    19.0,
                    color::MUTED,
                ));
                let mut state = lesson.practice_state(id, activity).clone();
                let (mut picked, mut result) = match &state {
                    PracticeState::Order { picked, result } => (picked.clone(), *result),
                    _ => (Vec::new(), None),
                };
                // Tampilkan urutan yang sedang dibangun.
                if !picked.is_empty() {
                    ui.label(theme::vt_text(
                        &format!("Urutanmu: {}", picked.join(" → ")),
                        20.0,
                        color::GOLD,
                    ));
                }
                ui.horizontal_wrapped(|ui| {
                    for item in items {
                        if picked.contains(item) {
                            continue;
                        }
                        if theme::pixel_chip(ui, item, false).clicked() {
                            picked.push(item.clone());
                        }
                    }
                    if !picked.is_empty()
                        && theme::pixel_button(ui, "↩ Hapus Terakhir", color::DUNGEON, color::RED).clicked()
                    {
                        picked.pop();
                    }
                });
                if picked.len() == items.len() {
                    if theme::pixel_button(ui, "Periksa Urutan", color::GREEN_DARK, color::CREAM)
                        .clicked()
                    {
                        let eval = evaluate_order_attempt(&picked, items);
                        record_attempt(app, &lesson.chunk_id, id, &all_ids);
                        result = Some(eval.is_correct);
                        state = PracticeState::Order { picked: picked.clone(), result };
                        lesson.practice.insert(id.clone(), state.clone());
                    }
                }
                if let PracticeState::Order { picked: cur, .. } = lesson.practice_state(id, activity) {
                    if *cur != picked {
                        *cur = picked.clone();
                    }
                }
                if let Some(ok) = result {
                    ui.label(theme::vt_text(
                        if ok { "✓ Urutan benar!" } else { "✗ Urutan belum tepat — coba lagi." },
                        20.0,
                        if ok { color::GREEN } else { color::RED },
                    ));
                    ui.label(theme::vt_text(why, 19.0, color::MUTED));
                }
            }
            LessonPracticeActivity::Match { id, question, pairs, why } => {
                ui.label(theme::vt_text(question, 21.0, color::CREAM));
                let mut state = lesson.practice_state(id, activity).clone();
                let (mut picks, mut result) = match &state {
                    PracticeState::Match { picks, result } => (picks.clone(), *result),
                    _ => (BTreeMap::new(), None),
                };
                let rights: Vec<String> = pairs.iter().map(|p| p.right.clone()).collect();
                for (li, pair) in pairs.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(theme::vt_text(&pair.left, 20.0, color::CREAM));
                        ui.label(theme::vt_text("→", 20.0, color::GOLD));
                        let mut current = picks.get(&li).cloned().unwrap_or_default();
                        egui::ComboBox::new(format!("match-{id}-{li}"), "Pasangan")
                            .selected_text(if current.is_empty() {
                                "Pilih…".to_string()
                            } else {
                                current.clone()
                            })
                            .show_ui(ui, |ui| {
                                for r in &rights {
                                    ui.selectable_value(&mut current, r.clone(), r.clone());
                                }
                            });
                        if !current.is_empty() {
                            picks.insert(li, current);
                        }
                    });
                }
                if picks.len() == pairs.len() {
                    if theme::pixel_button(ui, "Periksa Pasangan", color::GREEN_DARK, color::CREAM)
                        .clicked()
                    {
                        let eval = evaluate_match_attempt(&picks, pairs);
                        record_attempt(app, &lesson.chunk_id, id, &all_ids);
                        result = Some(eval.is_correct);
                        state = PracticeState::Match { picks: picks.clone(), result };
                        lesson.practice.insert(id.clone(), state.clone());
                    }
                }
                if let PracticeState::Match { picks: cur, .. } = lesson.practice_state(id, activity) {
                    if *cur != picks {
                        *cur = picks.clone();
                    }
                }
                if let Some(ok) = result {
                    ui.label(theme::vt_text(
                        if ok { "✓ Semua pasangan benar!" } else { "✗ Ada pasangan yang salah — coba lagi." },
                        20.0,
                        if ok { color::GREEN } else { color::RED },
                    ));
                    ui.label(theme::vt_text(why, 19.0, color::MUTED));
                }
            }
            LessonPracticeActivity::Unknown => {
                ui.label(theme::vt_text("Latihan tidak dikenal.", 20.0, color::MUTED));
            }
        }
    }
}

fn record_attempt(app: &mut App, chunk_id: &str, activity_id: &str, all_ids: &[String]) {
    app.apply(Action::RecordChunkPracticeAttempt {
        chunk_id: chunk_id.to_string(),
        activity_id: activity_id.to_string(),
        total_practice_ids: Some(all_ids.to_vec()),
    });
}

fn show_real_task(app: &mut App, ui: &mut Ui, lesson: &mut LessonUi) {
    let Some(chunk) = get_lesson_chunk_by_id(&lesson.chunk_id) else {
        return;
    };
    ui.label(theme::pixel_text("TUGAS NYATA", 11.0, color::GOLD));
    if let Some(task) = &chunk.real_task {
        ui.label(theme::vt_text(task, 21.0, color::CREAM));
    }
    ui.add_space(4.0);
    let done = app
        .state
        .task_done
        .get(&lesson.chunk_id)
        .copied()
        .unwrap_or(false);
    let mut check = done;
    if ui.checkbox(&mut check, "Tugas nyata sudah saya kerjakan").clicked() {
        app.apply(Action::ToggleChunkRealTaskDone {
            chunk_id: lesson.chunk_id.clone(),
            done: Some(check),
        });
    }
    ui.add_space(6.0);
    ui.label(theme::pixel_text("RINGKASAN", 11.0, color::GOLD));
    ui.label(theme::vt_text(&chunk.summary_prompt, 20.0, color::MUTED));
    let saved = app
        .state
        .summaries
        .get(&lesson.chunk_id)
        .cloned()
        .unwrap_or_default();
    if lesson.summary_text.is_empty() && !saved.is_empty() {
        lesson.summary_text = saved;
    }
    let resp = ui.add(
        egui::TextEdit::multiline(&mut lesson.summary_text)
            .font(egui::FontId::new(
                20.0,
                egui::FontFamily::Name(theme::FONT_VT.into()),
            ))
            .desired_width(ui.available_width())
            .desired_rows(3)
            .hint_text("Tulis ringkasanmu dengan kata-katamu sendiri…"),
    );
    if resp.lost_focus() && !lesson.summary_text.trim().is_empty() {
        app.apply(Action::SetChunkSummary {
            chunk_id: lesson.chunk_id.clone(),
            summary: lesson.summary_text.trim().to_string(),
        });
    }
}

fn show_finish(app: &mut App, ui: &mut Ui, lesson: &mut LessonUi) {
    let Some(chunk) = get_lesson_chunk_by_id(&lesson.chunk_id) else {
        return;
    };
    let done = is_chunk_complete(chunk, &app.state.lesson_progress);
    ui.label(theme::pixel_text("SELESAI", 11.0, color::GOLD));
    if lesson.completed {
        ui.label(theme::vt_text(
            "Chunk selesai! Kartu review sudah dijadwalkan ke Ulasan Harian.",
            22.0,
            color::GREEN,
        ));
        return;
    }
    if done {
        ui.label(theme::vt_text(
            "Semua latihan sudah dicoba — chunk siap diselesaikan.",
            21.0,
            color::GREEN,
        ));
        if theme::pixel_button(ui, "✓ Selesaikan Chunk", color::GREEN_DARK, color::CREAM)
            .clicked()
        {
            app.apply(Action::CompleteLessonChunk {
                chunk_id: lesson.chunk_id.clone(),
                review_items: chunk.review_items.clone(),
            });
            lesson.completed = true;
        }
    } else {
        let attempted = app
            .state
            .lesson_progress
            .get(&lesson.chunk_id)
            .map(|p| p.attempted_activities.len())
            .unwrap_or(0);
        ui.label(theme::vt_text(
            &format!(
                "Belum semua latihan dicoba ({}/{}). Kembali ke halaman Latihan.",
                attempted,
                chunk.practice.len()
            ),
            20.0,
            color::MUTED,
        ));
    }
}

/// Navigasi halaman: Sebelumnya / Lanjut / Tutup.
fn show_lesson_nav(app: &mut App, ui: &mut Ui, lesson: &mut LessonUi, close: &mut bool) {
    let Some(chunk) = get_lesson_chunk_by_id(&lesson.chunk_id) else {
        if theme::pixel_button(ui, "Tutup", color::GREEN_DARK, color::CREAM).clicked() {
            *close = true;
        }
        return;
    };
    let _ = app;
    let total = LessonUi::page_count(chunk);
    ui.horizontal(|ui| {
        if lesson.page > 0
            && theme::pixel_button(ui, "← Sebelumnya", color::DUNGEON, color::CREAM).clicked()
        {
            lesson.page -= 1;
        }
        ui.label(theme::vt_text(
            &format!("Halaman {}/{}", lesson.page + 1, total),
            19.0,
            color::MUTED,
        ));
        if lesson.page + 1 < total
            && theme::pixel_button(ui, "Lanjut →", color::GREEN_DARK, color::CREAM).clicked()
        {
            lesson.page += 1;
        }
        if lesson.page + 1 >= total
            && theme::pixel_button(ui, "Tutup", color::GREEN_DARK, color::CREAM).clicked()
        {
            *close = true;
        }
    });
}
