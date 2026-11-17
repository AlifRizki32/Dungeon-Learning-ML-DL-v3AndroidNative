//! Game state — port `useGameStore.ts` v2 (bagian state persisten + migrasi).
//!
//! Hanya state yang dipersist (41 field, kunci `dungeon-ml-dl:v1`); state UI
//! transien (`currentView`, `selectedTopicId`, `levelUpModal`, dll.) tinggal
//! di `dungeon-app`, bukan di sini. Level-up dikembalikan sebagai `Effect`
//! oleh reducer, bukan disimpan di state.
//!
//! Migrasi memakai parsing lenient per-field: field yang tipenya tidak cocok
//! jatuh ke bawaan (mirip spread `...rawState` di TS, yang tidak pernah
//! gagal total karena satu field rusak); state korup total tetap gagal
//! di validator sebelum sampai ke migrasi.

use crate::camp::{is_valid_floor_node_id, validate_and_parse_save_json, StudySession};
use crate::lesson_engine::{
    clamp_interval_scale, LessonChunkProgress, ReviewQueueEntry,
};
use crate::spellbook::{get_initial_boss_potions, get_topic_content, get_tower_relic_id, QuestionStat};
use dungeon_data::content::floor6_content;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

/// Versi state saat ini (v3).
pub const CURRENT_VERSION: i64 = 3;
/// Kunci penyimpanan v2 (dipertahankan agar save lama tetap terbaca).
pub const STORAGE_KEY: &str = "dungeon-ml-dl:v1";

/// Progres pertarungan boss satu lantai.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BossProgressState {
    pub cleared_phases: Vec<u32>,
    pub potions: u32,
    pub last_variant_by_phase: BTreeMap<u32, u8>,
}

impl Default for BossProgressState {
    fn default() -> Self {
        Self {
            cleared_phases: Vec::new(),
            potions: 2,
            last_variant_by_phase: BTreeMap::new(),
        }
    }
}

/// XP Dojo yang sudah didapat hari ini.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DojoDailyXp {
    pub date: String,
    pub xp: i64,
}

impl Default for DojoDailyXp {
    fn default() -> Self {
        Self {
            date: String::new(),
            xp: 0,
        }
    }
}

/// Pengaturan review.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub interval_scale: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Self { interval_scale: 1.0 }
    }
}

/// Seluruh state persisten game (41 field, urutan sama seperti export v2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GameState {
    pub version: i64,
    pub has_started: bool,
    pub player_name: String,
    pub avatar_id: String,
    pub xp: i64,
    pub streak: i64,
    pub last_active_date: Option<String>,
    pub completed_rooms: Vec<String>,
    pub defeated_bosses: Vec<String>,
    pub completed_tower_quests: Vec<String>,
    pub tower_steps: BTreeMap<String, BTreeMap<String, bool>>,
    pub tower_completed: Vec<String>,
    pub reflections: BTreeMap<String, String>,
    pub specialization: Option<String>,
    pub certificate_issued_at: Option<String>,
    pub relics: Vec<String>,
    pub badges: Vec<String>,
    pub quiz_best_scores: BTreeMap<String, i64>,
    pub best_score_by_topic: BTreeMap<String, i64>,
    pub quiz_bonus_claimed_rooms: Vec<String>,
    pub echo_bonus_claimed_rooms: Vec<String>,
    pub spells_unlocked: Vec<String>,
    pub question_stats: BTreeMap<String, QuestionStat>,
    pub boss_progress: BTreeMap<u32, BossProgressState>,
    /// `dojoDailyXP` — camelCase serde akan memberi "dojoDailyXp", jadi
    /// perlu rename eksplisit agar cocok dengan format save v2.
    #[serde(rename = "dojoDailyXP")]
    pub dojo_daily_xp: DojoDailyXp,
    pub evidence_notes: BTreeMap<String, String>,
    pub study_sessions: Vec<StudySession>,
    pub weekly_target_hours: f64,
    pub claimed_weekly_bonus_weeks: Vec<String>,
    pub active_floor_id: u32,
    pub active_node_by_floor: BTreeMap<u32, String>,
    pub sound_enabled: bool,
    pub sfx_volume: f64,
    pub music_enabled: bool,
    pub music_volume: f64,
    pub reduce_motion: bool,
    pub lesson_progress: BTreeMap<String, LessonChunkProgress>,
    pub summaries: BTreeMap<String, String>,
    pub task_done: BTreeMap<String, bool>,
    pub review_queue: BTreeMap<String, ReviewQueueEntry>,
    pub settings: Settings,
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            version: CURRENT_VERSION,
            has_started: false,
            player_name: "Petualang Tensor".to_string(),
            avatar_id: "alchemist".to_string(),
            xp: 0,
            streak: 0,
            last_active_date: None,
            completed_rooms: Vec::new(),
            defeated_bosses: Vec::new(),
            completed_tower_quests: Vec::new(),
            tower_steps: BTreeMap::new(),
            tower_completed: Vec::new(),
            reflections: BTreeMap::new(),
            specialization: None,
            certificate_issued_at: None,
            relics: Vec::new(),
            badges: Vec::new(),
            quiz_best_scores: BTreeMap::new(),
            best_score_by_topic: BTreeMap::new(),
            quiz_bonus_claimed_rooms: Vec::new(),
            echo_bonus_claimed_rooms: Vec::new(),
            spells_unlocked: Vec::new(),
            question_stats: BTreeMap::new(),
            boss_progress: create_initial_boss_progress_map(),
            dojo_daily_xp: DojoDailyXp::default(),
            evidence_notes: BTreeMap::new(),
            study_sessions: Vec::new(),
            weekly_target_hours: 9.0,
            claimed_weekly_bonus_weeks: Vec::new(),
            active_floor_id: 1,
            active_node_by_floor: [
                (1, "topic-1".to_string()),
                (2, "topic-7".to_string()),
                (3, "topic-12".to_string()),
                (4, "topic-18".to_string()),
                (5, "topic-21".to_string()),
            ]
            .into_iter()
            .collect(),
            sound_enabled: false,
            sfx_volume: 0.75,
            music_enabled: false,
            music_volume: 0.35,
            reduce_motion: false,
            lesson_progress: BTreeMap::new(),
            summaries: BTreeMap::new(),
            task_done: BTreeMap::new(),
            review_queue: BTreeMap::new(),
            settings: Settings::default(),
        }
    }
}

/// Progres boss bawaan lantai 1-5 (ramuan 2/2/3/2/3).
pub fn create_default_boss_progress_for_floor(floor_id: u32) -> BossProgressState {
    BossProgressState {
        cleared_phases: Vec::new(),
        potions: get_initial_boss_potions(floor_id),
        last_variant_by_phase: BTreeMap::new(),
    }
}

pub fn create_initial_boss_progress_map() -> BTreeMap<u32, BossProgressState> {
    (1..=5u32)
        .map(|f| (f, create_default_boss_progress_for_floor(f)))
        .collect()
}

// ============================================================================
// HELPER PARSING LENIENT
// ============================================================================

fn as_finite_f64(v: &Value) -> Option<f64> {
    v.as_f64().filter(|x| x.is_finite())
}

fn as_i64(v: &Value) -> Option<i64> {
    as_finite_f64(v).map(|x| x.floor() as i64)
}

fn as_u32(v: &Value) -> Option<u32> {
    as_finite_f64(v).and_then(|x| {
        let f = x.floor();
        if f >= 0.0 && f <= u32::MAX as f64 {
            Some(f as u32)
        } else {
            None
        }
    })
}

fn get_string(map: &Map<String, Value>, key: &str) -> Option<String> {
    map.get(key).and_then(Value::as_str).map(str::to_string)
}

fn get_bool(map: &Map<String, Value>, key: &str) -> Option<bool> {
    map.get(key).and_then(Value::as_bool)
}

fn get_i64(map: &Map<String, Value>, key: &str) -> Option<i64> {
    map.get(key).and_then(as_i64)
}

fn get_f64(map: &Map<String, Value>, key: &str) -> Option<f64> {
    map.get(key).and_then(as_finite_f64)
}

fn get_string_vec(map: &Map<String, Value>, key: &str) -> Vec<String> {
    match map.get(key).and_then(Value::as_array) {
        Some(arr) => arr
            .iter()
            .filter_map(|x| x.as_str().map(str::to_string))
            .collect(),
        None => Vec::new(),
    }
}

fn get_string_map(map: &Map<String, Value>, key: &str) -> BTreeMap<String, String> {
    match map.get(key).and_then(Value::as_object) {
        Some(obj) => obj
            .iter()
            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
            .collect(),
        None => BTreeMap::new(),
    }
}

fn truncate_chars(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

fn round2_clamp01(v: f64) -> f64 {
    ((v * 100.0).round() / 100.0).clamp(0.0, 1.0)
}

// ============================================================================
// NORMALISASI SATU PROGRES BOSS
// ============================================================================

fn normalize_single_boss_progress(raw: Option<&Value>, floor_id: u32) -> BossProgressState {
    let default_potions = get_initial_boss_potions(floor_id);
    let Some(obj) = raw.and_then(Value::as_object) else {
        return BossProgressState {
            cleared_phases: Vec::new(),
            potions: default_potions,
            last_variant_by_phase: BTreeMap::new(),
        };
    };
    let cleared_phases: Vec<u32> = obj
        .get("clearedPhases")
        .and_then(Value::as_array)
        .map(|arr| arr.iter().filter_map(as_u32).collect())
        .unwrap_or_default();
    let potions: u32 = obj
        .get("potions")
        .and_then(as_u32)
        .unwrap_or(default_potions);
    let mut last_variant_by_phase: BTreeMap<u32, u8> = BTreeMap::new();
    if let Some(variants) = obj.get("lastVariantByPhase").and_then(Value::as_object) {
        for (k, v) in variants {
            if let (Ok(phase), Some(idx)) = (k.parse::<u32>(), as_finite_f64(v)) {
                if idx >= 0.0 && idx <= u8::MAX as f64 {
                    last_variant_by_phase.insert(phase, idx.floor() as u8);
                }
            }
        }
    }
    BossProgressState {
        cleared_phases,
        potions,
        last_variant_by_phase,
    }
}

// ============================================================================
// MIGRASI v1/v2 -> v3
// ============================================================================

/// Migrasi tanpa kehilangan data dari state v1/v2 ke v3:
/// - Menurunkan mantra dari ruangan yang sudah selesai.
/// - BossProgress datar v2 -> dipindah ke lantai 1 + bawaan lantai 2-5.
/// - `tower-N` lawas -> `mN`; quest selesai mendapat semua langkah
///   dicentang + reliknya.
/// - Normalisasi catatan bukti/refleksi/ringkasan, progres lesson,
///   review queue, node aktif, volume, dan skala interval.
pub fn migrate_state_to_v3(raw_state: &Map<String, Value>) -> GameState {
    let mut state = GameState::default();

    // --- Spread mentah (field yang tidak dinormalisasi di bawah) ---
    if let Some(b) = get_bool(raw_state, "hasStarted") {
        state.has_started = b;
    }
    if let Some(s) = get_string(raw_state, "playerName") {
        state.player_name = s;
    }
    if let Some(s) = get_string(raw_state, "avatarId") {
        state.avatar_id = s;
    }
    if let Some(x) = get_i64(raw_state, "xp") {
        state.xp = x;
    }
    if let Some(x) = get_i64(raw_state, "streak") {
        state.streak = x;
    }
    match raw_state.get("lastActiveDate") {
        Some(Value::Null) => state.last_active_date = None,
        Some(Value::String(s)) => state.last_active_date = Some(s.clone()),
        _ => {}
    }
    state.defeated_bosses = get_string_vec(raw_state, "defeatedBosses");
    state.badges = get_string_vec(raw_state, "badges");
    state.quiz_bonus_claimed_rooms = get_string_vec(raw_state, "quizBonusClaimedRooms");
    state.echo_bonus_claimed_rooms = get_string_vec(raw_state, "echoBonusClaimedRooms");
    if let Some(obj) = raw_state.get("dojoDailyXP").and_then(Value::as_object) {
        state.dojo_daily_xp = DojoDailyXp {
            date: obj
                .get("date")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            xp: obj.get("xp").and_then(as_i64).unwrap_or(0),
        };
    }
    if let Some(arr) = raw_state.get("studySessions").and_then(Value::as_array) {
        state.study_sessions = arr
            .iter()
            .filter_map(|v| serde_json::from_value::<StudySession>(v.clone()).ok())
            .collect();
    }
    if let Some(h) = get_f64(raw_state, "weeklyTargetHours") {
        state.weekly_target_hours = h;
    }
    state.claimed_weekly_bonus_weeks = get_string_vec(raw_state, "claimedWeeklyBonusWeeks");
    if let Some(b) = get_bool(raw_state, "soundEnabled") {
        state.sound_enabled = b;
    }
    if let Some(b) = get_bool(raw_state, "reduceMotion") {
        state.reduce_motion = b;
    }

    // --- completedRooms + mantra turunan ---
    let completed_rooms = get_string_vec(raw_state, "completedRooms");
    state.completed_rooms = completed_rooms.clone();
    let mut derived_spells: Vec<String> = get_string_vec(raw_state, "spellsUnlocked");
    for room_id in &completed_rooms {
        if let Some(content) = get_topic_content(room_id) {
            let spell_id = content.topic.spell.id.clone();
            if !derived_spells.contains(&spell_id) {
                derived_spells.push(spell_id);
            }
        }
    }
    state.spells_unlocked = derived_spells;

    // --- Skor terbaik: bestScoreByTopic menang atas quizBestScores ---
    let prev_best_scores: BTreeMap<String, i64> = {
        let source = raw_state
            .get("bestScoreByTopic")
            .and_then(Value::as_object)
            .or_else(|| raw_state.get("quizBestScores").and_then(Value::as_object));
        match source {
            Some(obj) => obj
                .iter()
                .filter_map(|(k, v)| as_i64(v).map(|n| (k.clone(), n)))
                .collect(),
            None => BTreeMap::new(),
        }
    };
    state.quiz_best_scores = prev_best_scores.clone();
    state.best_score_by_topic = prev_best_scores;

    // --- bossProgress: datar v2 vs berkunci lantai v3 ---
    let raw_boss = raw_state.get("bossProgress");
    let boss_map: BTreeMap<u32, BossProgressState> = match raw_boss.and_then(Value::as_object) {
        Some(obj) if obj.contains_key("clearedPhases") || obj.contains_key("potions") => {
            // v2 datar -> lantai 1, sisanya bawaan.
            (1..=5u32)
                .map(|f| {
                    let entry = if f == 1 {
                        normalize_single_boss_progress(raw_boss, 1)
                    } else {
                        normalize_single_boss_progress(None, f)
                    };
                    (f, entry)
                })
                .collect()
        }
        Some(obj) => (1..=5u32)
            .map(|f| {
                let key = f.to_string();
                let entry = normalize_single_boss_progress(obj.get(&key), f);
                (f, entry)
            })
            .collect(),
        None => create_initial_boss_progress_map(),
    };
    state.boss_progress = boss_map;

    // --- evidenceNotes (<= 300 karakter) ---
    state.evidence_notes = get_string_map(raw_state, "evidenceNotes")
        .into_iter()
        .map(|(k, v)| (k, truncate_chars(&v, 300)))
        .collect();

    // --- towerSteps (hanya boolean true) ---
    let mut tower_steps: BTreeMap<String, BTreeMap<String, bool>> = BTreeMap::new();
    if let Some(obj) = raw_state.get("towerSteps").and_then(Value::as_object) {
        for (q_id, step_obj) in obj {
            if let Some(inner) = step_obj.as_object() {
                let mut clean: BTreeMap<String, bool> = BTreeMap::new();
                for (s_id, val) in inner {
                    if val.as_bool() == Some(true) {
                        clean.insert(s_id.clone(), true);
                    }
                }
                tower_steps.insert(q_id.clone(), clean);
            }
        }
    }

    // --- towerCompleted / completedTowerQuests (legacy tower-N -> mN) ---
    let mut tower_set: Vec<String> = Vec::new();
    let mut push_tower = |items: &Vec<String>| {
        for item in items {
            let q_id = if item.starts_with("tower-") {
                let num = item.trim_start_matches("tower-");
                if num.bytes().all(|b| b.is_ascii_digit()) && !num.is_empty() {
                    format!("m{num}")
                } else {
                    item.clone()
                }
            } else {
                item.clone()
            };
            if floor6_content().quests.iter().any(|q| q.id == q_id)
                && !tower_set.contains(&q_id)
            {
                tower_set.push(q_id);
            }
        }
    };
    push_tower(&get_string_vec(raw_state, "towerCompleted"));
    push_tower(&get_string_vec(raw_state, "completedTowerQuests"));

    // --- Relik: yang sudah ada + relik quest Menara yang selesai ---
    let mut relics = get_string_vec(raw_state, "relics");
    for q_id in &tower_set {
        if let Some(quest) = floor6_content().quests.iter().find(|q| &q.id == q_id) {
            let entry = tower_steps.entry(q_id.clone()).or_default();
            for s in &quest.steps {
                entry.insert(s.id.clone(), true);
            }
            let relic_id = get_tower_relic_id(&quest.id);
            if !relics.contains(&relic_id) {
                relics.push(relic_id);
            }
        }
    }
    state.relics = relics;
    state.completed_tower_quests = tower_set.clone();
    state.tower_completed = tower_set;
    state.tower_steps = tower_steps;

    // --- reflections (<= 500 karakter) ---
    state.reflections = get_string_map(raw_state, "reflections")
        .into_iter()
        .map(|(k, v)| (k, truncate_chars(&v, 500)))
        .collect();

    // --- specialization / certificateIssuedAt ---
    state.specialization = match get_string(raw_state, "specialization") {
        Some(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
        _ => None,
    };
    state.certificate_issued_at = match get_string(raw_state, "certificateIssuedAt") {
        Some(s) if !s.trim().is_empty() => Some(s),
        _ => None,
    };

    // --- questionStats (entri valid saja) ---
    if let Some(obj) = raw_state.get("questionStats").and_then(Value::as_object) {
        let mut stats: BTreeMap<String, QuestionStat> = BTreeMap::new();
        for (k, v) in obj {
            if let Ok(stat) = serde_json::from_value::<QuestionStat>(v.clone()) {
                stats.insert(k.clone(), stat);
            }
        }
        state.question_stats = stats;
    }

    // --- activeFloorId (1-6) ---
    if let Some(n) = get_i64(raw_state, "activeFloorId") {
        if (1..=6).contains(&n) {
            state.active_floor_id = n as u32;
        }
    }

    // --- activeNodeByFloor (bawaan + override valid) ---
    let mut nodes = state.active_node_by_floor.clone();
    if let Some(obj) = raw_state.get("activeNodeByFloor").and_then(Value::as_object) {
        for (f_key, node_val) in obj {
            if let Ok(floor_num) = f_key.parse::<u32>() {
                if (1..=5).contains(&floor_num) {
                    if let Some(node) = node_val.as_str() {
                        if is_valid_floor_node_id(floor_num, node) {
                            nodes.insert(floor_num, node.to_string());
                        }
                    }
                }
            }
        }
    }
    state.active_node_by_floor = nodes;

    // --- lessonProgress ---
    if let Some(obj) = raw_state.get("lessonProgress").and_then(Value::as_object) {
        let mut progress: BTreeMap<String, LessonChunkProgress> = BTreeMap::new();
        for (c_id, val) in obj {
            if let Some(o) = val.as_object() {
                let done = o.get("done").and_then(Value::as_bool).unwrap_or(false);
                let attempts = o
                    .get("attempts")
                    .and_then(as_finite_f64)
                    .filter(|n| *n >= 0.0)
                    .map(|n| n.floor() as u32)
                    .unwrap_or(0);
                let activities: Vec<String> = o
                    .get("attemptedActivities")
                    .and_then(Value::as_array)
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|x| x.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default();
                progress.insert(
                    c_id.clone(),
                    LessonChunkProgress {
                        done,
                        attempts,
                        attempted_activities: activities,
                    },
                );
            }
        }
        state.lesson_progress = progress;
    }

    // --- summaries (<= 400 karakter) ---
    state.summaries = get_string_map(raw_state, "summaries")
        .into_iter()
        .map(|(k, v)| (k, truncate_chars(&v, 400)))
        .collect();

    // --- taskDone (boolean saja) ---
    if let Some(obj) = raw_state.get("taskDone").and_then(Value::as_object) {
        state.task_done = obj
            .iter()
            .filter_map(|(k, v)| v.as_bool().map(|b| (k.clone(), b)))
            .collect();
    }

    // --- reviewQueue (box 0-4, due YYYY-MM-DD) ---
    if let Some(obj) = raw_state.get("reviewQueue").and_then(Value::as_object) {
        let mut queue: BTreeMap<String, ReviewQueueEntry> = BTreeMap::new();
        for (item_id, val) in obj {
            if let Some(o) = val.as_object() {
                let box_n = o.get("box").and_then(as_finite_f64);
                let due = o.get("due").and_then(Value::as_str);
                if let (Some(box_n), Some(due)) = (box_n, due) {
                    let due_t = due.trim();
                    if crate::camp::is_valid_date_key(due_t) {
                        queue.insert(
                            item_id.clone(),
                            ReviewQueueEntry {
                                box_: (box_n.floor() as i64).clamp(0, 4) as u8,
                                due: due_t.to_string(),
                            },
                        );
                    }
                }
            }
        }
        state.review_queue = queue;
    }

    // --- settings.intervalScale ---
    if let Some(obj) = raw_state.get("settings").and_then(Value::as_object) {
        let scale = obj
            .get("intervalScale")
            .and_then(as_finite_f64)
            .unwrap_or(1.0);
        state.settings.interval_scale = clamp_interval_scale(scale);
    }

    // --- musik & sfx ---
    if let Some(b) = get_bool(raw_state, "musicEnabled") {
        state.music_enabled = b;
    }
    if let Some(v) = get_f64(raw_state, "musicVolume") {
        state.music_volume = round2_clamp01(v);
    }
    if let Some(v) = get_f64(raw_state, "sfxVolume") {
        state.sfx_volume = round2_clamp01(v);
    }

    state.version = CURRENT_VERSION;
    state
}

/// Alias `migrate_state_to_v3` (kompatibilitas v2: `migrateV1ToV2State`).
pub fn migrate_v1_to_v2_state(raw_state: &Map<String, Value>) -> GameState {
    migrate_state_to_v3(raw_state)
}

// ============================================================================
// EXPORT / IMPORT SAVE JSON
// ============================================================================

/// Mengekspor state menjadi string JSON save v2:
/// `{ version: 3, exportedAt, state: {...41 field...} }`.
/// `exported_at` dipasok pemanggil (core tidak membaca jam sistem).
pub fn export_save_json(state: &GameState, exported_at: &str) -> String {
    let payload = serde_json::json!({
        "version": CURRENT_VERSION,
        "exportedAt": exported_at,
        "state": state,
    });
    serde_json::to_string_pretty(&payload).unwrap_or_default()
}

/// Mengimpor string JSON save: validasi (sanitasi per-field) lalu migrasi
/// ke v3, dengan `hasStarted: true`. `Err` berisi pesan galat berbahasa
/// Indonesia; pemanggil tetap memakai state lama bila impor gagal.
pub fn import_save_json(raw_json: &str) -> Result<GameState, String> {
    let sanitized = validate_and_parse_save_json(raw_json)?;
    let mut state = migrate_state_to_v3(&sanitized);
    state.has_started = true;
    state.version = CURRENT_VERSION;
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camp::StudySessionSource;
    use crate::lesson_engine::ReviewRating;

    fn str_vec(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn migrate_v2_flat_boss_progress_to_v3() {
        // Port spellbookEngine.test.ts (bagian migrasi v2->v3).
        let raw = serde_json::json!({
            "version": 2,
            "completedRooms": ["topic-1", "topic-7", "topic-12", "topic-18", "topic-21"],
            "evidenceNotes": { "topic-21": "Repo GitHub dengan tag v1.0" },
            "bossProgress": {
                "clearedPhases": [1, 2, 3, 4, 5, 6],
                "potions": 1,
                "lastVariantByPhase": { "1": 0 }
            }
        });
        let migrated = migrate_state_to_v3(raw.as_object().unwrap());
        assert_eq!(migrated.version, 3);
        for s in ["s1", "s7", "s12", "s18", "s21"] {
            assert!(
                migrated.spells_unlocked.contains(&s.to_string()),
                "mantra {s} harus terbuka dari ruangan selesai"
            );
        }
        assert_eq!(
            migrated
                .evidence_notes
                .get("topic-21")
                .map(String::as_str),
            Some("Repo GitHub dengan tag v1.0")
        );
        assert_eq!(
            migrated.boss_progress[&1].cleared_phases,
            vec![1, 2, 3, 4, 5, 6]
        );
        assert_eq!(migrated.boss_progress[&1].potions, 1);
        assert!(migrated.boss_progress[&2].cleared_phases.is_empty());
        assert_eq!(migrated.boss_progress[&2].potions, 2);
        assert!(migrated.boss_progress[&3].cleared_phases.is_empty());
        assert_eq!(migrated.boss_progress[&3].potions, 3);
        assert!(migrated.boss_progress[&4].cleared_phases.is_empty());
        assert_eq!(migrated.boss_progress[&4].potions, 2);
        assert!(migrated.boss_progress[&5].cleared_phases.is_empty());
        assert_eq!(migrated.boss_progress[&5].potions, 3);
    }

    fn p06_full_state() -> GameState {
        let mut state = GameState::default();
        state.version = 3;
        state.has_started = true;
        state.player_name = "Ratu Tensor Nusantara".to_string();
        state.avatar_id = "ranger".to_string();
        state.xp = 1450;
        state.streak = 5;
        state.last_active_date = Some("2026-11-03".to_string());
        state.completed_rooms = str_vec(&["topic-1", "topic-6", "topic-7", "topic-12", "topic-21"]);
        state.defeated_bosses = str_vec(&["boss-1"]);
        state.completed_tower_quests = str_vec(&["m1"]);
        state.tower_steps = [
            (
                "m1".to_string(),
                [
                    ("m1s1".to_string(), true),
                    ("m1s2".to_string(), true),
                    ("m1s3".to_string(), true),
                    ("m1s4".to_string(), true),
                    ("m1s5".to_string(), true),
                ]
                .into_iter()
                .collect(),
            ),
            (
                "m2".to_string(),
                [("m2s1".to_string(), true)].into_iter().collect(),
            ),
        ]
        .into_iter()
        .collect();
        state.tower_completed = str_vec(&["m1"]);
        state.reflections = [
            (
                "p1".to_string(),
                "Refleksi mendalam tentang fondasi matematika dan aljabar linear."
                    .to_string(),
            ),
            (
                "p2".to_string(),
                "Catatan evaluasi diri proyek ML.".to_string(),
            ),
        ]
        .into_iter()
        .collect();
        state.specialization = Some("Embedded ML / TinyML & Edge AI".to_string());
        state.certificate_issued_at = Some("2026-11-03T10:00:00.000Z".to_string());
        state.relics = str_vec(&["relic-1", "relic-g1", "relic-m1"]);
        state.badges = str_vec(&["badge-1"]);
        state.quiz_best_scores = [
            ("topic-1".to_string(), 100),
            ("topic-6".to_string(), 80),
        ]
        .into_iter()
        .collect();
        state.best_score_by_topic = state.quiz_best_scores.clone();
        state.quiz_bonus_claimed_rooms = str_vec(&["topic-1"]);
        state.echo_bonus_claimed_rooms = str_vec(&["topic-6"]);
        state.spells_unlocked = str_vec(&["s1", "s6", "s7", "s12", "s21"]);
        state.question_stats = [(
            "t1q1".to_string(),
            QuestionStat {
                seen: 3,
                wrong: 1,
                correct_streak: 2,
            },
        )]
        .into_iter()
        .collect();
        state.boss_progress = [
            (
                1,
                BossProgressState {
                    cleared_phases: vec![1, 2, 3, 4, 5, 6],
                    potions: 1,
                    last_variant_by_phase: [(1, 0), (2, 1)].into_iter().collect(),
                },
            ),
            (
                2,
                BossProgressState {
                    cleared_phases: vec![1, 2],
                    potions: 2,
                    last_variant_by_phase: [(1, 0)].into_iter().collect(),
                },
            ),
            (
                3,
                BossProgressState {
                    cleared_phases: vec![],
                    potions: 3,
                    last_variant_by_phase: BTreeMap::new(),
                },
            ),
            (
                4,
                BossProgressState {
                    cleared_phases: vec![],
                    potions: 2,
                    last_variant_by_phase: BTreeMap::new(),
                },
            ),
            (
                5,
                BossProgressState {
                    cleared_phases: vec![1],
                    potions: 2,
                    last_variant_by_phase: [(1, 1)].into_iter().collect(),
                },
            ),
        ]
        .into_iter()
        .collect();
        state.dojo_daily_xp = DojoDailyXp {
            date: "2026-11-03".to_string(),
            xp: 14,
        };
        state.evidence_notes = [
            (
                "topic-21".to_string(),
                "Repo Git dengan 12 commit rapi dan pytest hijau.".to_string(),
            ),
            (
                "21".to_string(),
                "Repo Git dengan 12 commit rapi dan pytest hijau.".to_string(),
            ),
            (
                "m1".to_string(),
                "Catatan bukti selesai membaca bab ISLP.".to_string(),
            ),
        ]
        .into_iter()
        .collect();
        state.study_sessions = vec![StudySession {
            id: "ses-test-1".to_string(),
            topic_id: "topic-1".to_string(),
            duration_minutes: 90.0,
            date: "2026-11-03".to_string(),
            note: Some("Latihan soal pecahan dan garis lurus".to_string()),
            source: StudySessionSource::Manual,
            created_at: 1762170000000,
        }];
        state.weekly_target_hours = 12.0;
        state.claimed_weekly_bonus_weeks = str_vec(&["2026-11-02"]);
        state.active_floor_id = 2;
        state.active_node_by_floor = [
            (1, "boss-1".to_string()),
            (2, "topic-8".to_string()),
            (3, "topic-13".to_string()),
            (4, "topic-19".to_string()),
            (5, "topic-22".to_string()),
        ]
        .into_iter()
        .collect();
        state.sound_enabled = true;
        state.sfx_volume = 0.6;
        state.music_enabled = true;
        state.music_volume = 0.25;
        state.reduce_motion = true;
        state.lesson_progress = [(
            "t1c1".to_string(),
            LessonChunkProgress {
                done: true,
                attempts: 3,
                attempted_activities: str_vec(&["t1c1p1", "t1c1p2", "t1c1p3"]),
            },
        )]
        .into_iter()
        .collect();
        state.summaries = [(
            "t1c1".to_string(),
            "Rata-rata adalah jumlah seluruh nilai dibagi banyaknya data.".to_string(),
        )]
        .into_iter()
        .collect();
        state.task_done = [
            ("t21c1".to_string(), true),
            ("m1c1".to_string(), true),
        ]
        .into_iter()
        .collect();
        state.review_queue = [(
            "t1c1r1".to_string(),
            ReviewQueueEntry {
                box_: 2,
                due: "2026-11-10".to_string(),
            },
        )]
        .into_iter()
        .collect();
        state.settings.interval_scale = 1.5;
        state
    }

    #[test]
    fn p06_export_reset_import_preserves_every_persisted_key() {
        let state = p06_full_state();
        let exported = export_save_json(&state, "2026-11-03T10:00:00.000Z");
        let exported_parsed: Value = serde_json::from_str(&exported).unwrap();

        // Reset total ke bawaan.
        let reset = GameState::default();
        assert_eq!(reset.xp, 0);
        assert!(reset.lesson_progress.is_empty());
        assert!(reset.summaries.is_empty());
        assert!(reset.review_queue.is_empty());

        // Impor save yang diekspor.
        let imported = import_save_json(&exported).unwrap();
        let re_exported = export_save_json(&imported, "2026-11-03T11:00:00.000Z");
        let re_parsed: Value = serde_json::from_str(&re_exported).unwrap();

        assert_eq!(re_parsed["version"], exported_parsed["version"]);
        assert_eq!(re_parsed["state"], exported_parsed["state"]);
        // ReviewRating dipakai di reducer; pastikan enum tersedia di modul ini.
        assert_eq!(ReviewRating::Ingat, ReviewRating::Ingat);
    }

    #[test]
    fn p06_import_v1_and_v2_shapes_migrates_without_data_loss() {
        // 1. Bentuk v1 (tanpa spellsUnlocked, hanya quizBestScores, id tower-1..7 lawas).
        let v1 = serde_json::json!({
            "version": 1,
            "state": {
                "playerName": "Veteran V1",
                "avatarId": "knight",
                "xp": 850,
                "streak": 3,
                "lastActiveDate": "2026-11-01",
                "completedRooms": ["topic-1", "topic-6", "topic-7"],
                "defeatedBosses": ["boss-1"],
                "completedTowerQuests": ["tower-1", "tower-2"],
                "quizBestScores": { "topic-1": 100, "topic-6": 80 },
                "relics": ["relic-1"],
                "badges": ["badge-1"]
            }
        })
        .to_string();
        let s1 = import_save_json(&v1).unwrap();
        assert_eq!(s1.player_name, "Veteran V1");
        assert_eq!(s1.xp, 850);
        assert_eq!(s1.completed_rooms, str_vec(&["topic-1", "topic-6", "topic-7"]));
        let mut spells = s1.spells_unlocked.clone();
        spells.sort();
        assert_eq!(spells, str_vec(&["s1", "s6", "s7"]));
        let mut towers = s1.tower_completed.clone();
        towers.sort();
        assert_eq!(towers, str_vec(&["m1", "m2"]));
        assert_eq!(s1.best_score_by_topic.get("topic-1"), Some(&100));
        assert!(s1.relics.contains(&"relic-m1".to_string()));
        assert!(s1.relics.contains(&"relic-m2".to_string()));

        // 2. Bentuk v2 (bossProgress datar).
        let v2 = serde_json::json!({
            "version": 2,
            "state": {
                "playerName": "Veteran V2",
                "xp": 1200,
                "completedRooms": ["topic-1", "topic-2"],
                "bossProgress": {
                    "clearedPhases": [1, 2, 3],
                    "potions": 1,
                    "lastVariantByPhase": { "1": 0, "2": 1 }
                },
                "evidenceNotes": { "21": "Catatan bukti v2" }
            }
        })
        .to_string();
        let s2 = import_save_json(&v2).unwrap();
        assert_eq!(s2.player_name, "Veteran V2");
        assert_eq!(s2.xp, 1200);
        assert_eq!(s2.boss_progress[&1].cleared_phases, vec![1, 2, 3]);
        assert_eq!(s2.boss_progress[&1].potions, 1);
        assert_eq!(s2.boss_progress[&1].last_variant_by_phase.get(&1), Some(&0));
        assert_eq!(s2.boss_progress[&1].last_variant_by_phase.get(&2), Some(&1));
        assert!(s2.boss_progress[&2].cleared_phases.is_empty());
        assert_eq!(
            s2.evidence_notes.get("21").map(String::as_str),
            Some("Catatan bukti v2")
        );
    }

    #[test]
    fn p06_dirty_payload_bounds_and_failed_imports_keep_save() {
        let dirty = serde_json::json!({
            "version": 3,
            "state": {
                "playerName": "A".repeat(50),
                "summaries": { "t1c1": "S".repeat(500), "badType": 12345 },
                "evidenceNotes": { "21": "E".repeat(400), "badNote": null },
                "reflections": { "q1": "R".repeat(600), "q2": "Valid reflection", "badRefl": 999 },
                "reviewQueue": {
                    "validCard": { "box": 4, "due": "2026-11-15" },
                    "clampedBox": { "box": 99, "due": "2026-11-16" },
                    "badDate": { "box": 2, "due": "not-a-date" }
                },
                "settings": { "intervalScale": 9.9 },
                "taskDone": { "t1c1": true, "t1c2": false, "t1c3": "not-a-bool" },
                "lessonProgress": {
                    "t1c1": { "done": true, "attempts": 3, "attemptedActivities": ["guess", 999, "p0"] },
                    "badEntry": "invalid"
                },
                "activeFloorId": 99,
                "activeNodeByFloor": {
                    "1": "topic-2",
                    "2": "topic-999",
                    "3": "topic-1",
                    "4": "boss-4"
                }
            }
        })
        .to_string();
        let imported = import_save_json(&dirty).unwrap();
        assert_eq!(imported.player_name.chars().count(), 28);
        assert_eq!(imported.summaries["t1c1"].chars().count(), 400);
        assert!(!imported.summaries.contains_key("badType"));
        assert_eq!(imported.evidence_notes["21"].chars().count(), 300);
        assert!(!imported.evidence_notes.contains_key("badNote"));
        assert_eq!(imported.reflections["q1"].chars().count(), 500);
        assert_eq!(imported.reflections["q2"], "Valid reflection");
        assert!(!imported.reflections.contains_key("badRefl"));
        assert_eq!(imported.review_queue["validCard"].box_, 4);
        assert_eq!(imported.review_queue["clampedBox"].box_, 4);
        assert!(!imported.review_queue.contains_key("badDate"));
        assert_eq!(imported.settings.interval_scale, 2.0);
        assert_eq!(imported.task_done.get("t1c1"), Some(&true));
        assert_eq!(imported.task_done.get("t1c2"), Some(&false));
        assert!(!imported.task_done.contains_key("t1c3"));
        assert_eq!(
            imported.lesson_progress["t1c1"].attempted_activities,
            str_vec(&["guess", "p0"])
        );
        assert!(!imported.lesson_progress.contains_key("badEntry"));
        assert_eq!(imported.active_floor_id, 1);
        assert_eq!(imported.active_node_by_floor[&1], "topic-2");
        assert_eq!(imported.active_node_by_floor[&2], "topic-7");
        assert_eq!(imported.active_node_by_floor[&3], "topic-12");
        assert_eq!(imported.active_node_by_floor[&4], "boss-4");

        // Impor gagal / oversized: tidak menghasilkan state baru — pemanggil
        // tetap memakai state lama (di Rust dijamin oleh Result).
        let before = export_save_json(&imported, "2026-11-03T10:00:00.000Z");
        let oversized = "x".repeat(1024 * 1024 + 10);
        let err = import_save_json(&oversized).unwrap_err();
        assert!(err.contains("1 MB"));
        assert!(import_save_json("{not valid json").is_err());
        let after = export_save_json(&imported, "2026-11-03T10:00:00.000Z");
        assert_eq!(after, before);
    }
}
