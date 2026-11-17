//! Spellbook engine — port `spellbookEngine.ts` v2.
//!
//! Meliputi: tema visual 28 mantra, mini-lab numerik (neuron, softmax,
//! quant, latensi), deck pertempuran + aturan gema, bobot Dojo, aturan
//! varian/pingsan boss, dan logika Lantai 6 (Menara Master).

use crate::rng::Rng;
use dungeon_data::content::{
    BossPhase, BossVariant, Difficulty, FloorContentFile, QuizQuestion, TopicContent,
    TowerQuestContent,
};
use dungeon_data::content::{floor6_content, floor_content};
use dungeon_data::roadmap::topic_by_id;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ============================================================================
// KONSTANTA
// ============================================================================

/// XP bonus menjawab soal gema dengan benar.
pub const ECHO_BONUS_XP: i64 = 5;
/// Bonus crit ruangan: +10% XP ruangan, sekali per ruangan.
pub const CRIT_ROOM_BONUS_RATE: f64 = 0.1;
/// XP per jawaban benar di Dojo.
pub const DOJO_CORRECT_XP: i64 = 2;
/// Batas XP Dojo per hari.
pub const DOJO_DAILY_XP_CAP: i64 = 20;
/// Ramuan boss awal (lantai 1, 2, 4).
pub const INITIAL_BOSS_POTIONS: u32 = 2;
/// Ramuan boss Lantai 3.
pub const FLOOR3_BOSS_POTIONS: u32 = 3;
/// Ramuan boss Lantai 5.
pub const FLOOR5_BOSS_POTIONS: u32 = 3;
/// Hati maksimal saat melawan boss.
pub const MAX_HEARTS: u32 = 3;

/// Ramuan awal boss per lantai (3 di Lantai 3 & 5, 2 di lantai lain).
pub fn get_initial_boss_potions(floor_id: u32) -> u32 {
    if floor_id == 3 {
        FLOOR3_BOSS_POTIONS
    } else if floor_id == 5 {
        FLOOR5_BOSS_POTIONS
    } else {
        INITIAL_BOSS_POTIONS
    }
}

/// Jumlah fase boss satu lantai (6 bila lantai tidak dikenal).
pub fn get_total_boss_phases(floor_id: u32) -> usize {
    match floor_content(floor_id) {
        Some(f) => f.boss.phases.len(),
        None => 6,
    }
}

// ============================================================================
// TEMA VISUAL MANTRA (28)
// ============================================================================

/// Tema visual satu mantra.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellVisualTheme {
    pub spell_id: &'static str,
    pub floor_id: u8,
    pub color: &'static str,
    pub bg_color: &'static str,
    pub anim_style: &'static str,
    pub rune_symbol: &'static str,
}

pub const SPELL_VISUALS: [SpellVisualTheme; 28] = [
    SpellVisualTheme { spell_id: "s1", floor_id: 1, color: "#38d864", bg_color: "#0a1714", anim_style: "beam", rune_symbol: "% / y=mx+c" },
    SpellVisualTheme { spell_id: "s2", floor_id: 1, color: "#3ac5eb", bg_color: "#0d1b2a", anim_style: "orbs", rune_symbol: "P(A|B)" },
    SpellVisualTheme { spell_id: "s3", floor_id: 1, color: "#f5b935", bg_color: "#291324", anim_style: "lightning", rune_symbol: "p < α" },
    SpellVisualTheme { spell_id: "s4", floor_id: 1, color: "#9b5de5", bg_color: "#16112b", anim_style: "grid", rune_symbol: "A·v = λv" },
    SpellVisualTheme { spell_id: "s5", floor_id: 1, color: "#e83b47", bg_color: "#291324", anim_style: "spiral", rune_symbol: "θ − η∇L" },
    SpellVisualTheme { spell_id: "s6", floor_id: 1, color: "#f4f1ea", bg_color: "#1f193d", anim_style: "code", rune_symbol: "df.groupby" },
    SpellVisualTheme { spell_id: "s7", floor_id: 2, color: "#1d8a3d", bg_color: "#0a1714", anim_style: "roots", rune_symbol: "Train|Val|Test" },
    SpellVisualTheme { spell_id: "s8", floor_id: 2, color: "#a87312", bg_color: "#241909", anim_style: "leaves", rune_symbol: "F1 = 2PR/(P+R)" },
    SpellVisualTheme { spell_id: "s9", floor_id: 2, color: "#94a3b8", bg_color: "#121b28", anim_style: "prisms", rune_symbol: "σ(z) · Gini" },
    SpellVisualTheme { spell_id: "s10", floor_id: 2, color: "#64748b", bg_color: "#161e29", anim_style: "forge", rune_symbol: "z = (x−μ)/s" },
    SpellVisualTheme { spell_id: "s11", floor_id: 2, color: "#5c4e99", bg_color: "#16112b", anim_style: "constellation", rune_symbol: "K-Means · PC1" },
    SpellVisualTheme { spell_id: "s12", floor_id: 3, color: "#b3aad1", bg_color: "#1a142e", anim_style: "synapse", rune_symbol: "z = w·x+b · ReLU" },
    SpellVisualTheme { spell_id: "s13", floor_id: 3, color: "#941923", bg_color: "#260d14", anim_style: "tensor", rune_symbol: "zero_grad · step" },
    SpellVisualTheme { spell_id: "s14", floor_id: 3, color: "#0284c7", bg_color: "#0c1f33", anim_style: "kernel", rune_symbol: "Conv2D · Pool" },
    SpellVisualTheme { spell_id: "s15", floor_id: 3, color: "#b45309", bg_color: "#241509", anim_style: "sequence", rune_symbol: "h_t = tanh(Wh)" },
    SpellVisualTheme { spell_id: "s16", floor_id: 3, color: "#fcd34d", bg_color: "#261f0a", anim_style: "attention", rune_symbol: "softmax(QK^T/√d)" },
    SpellVisualTheme { spell_id: "s17", floor_id: 3, color: "#7e22ce", bg_color: "#1b0d2e", anim_style: "schedule", rune_symbol: "AdamW · CosineLR" },
    SpellVisualTheme { spell_id: "s18", floor_id: 4, color: "#0891b2", bg_color: "#08202b", anim_style: "pipeline", rune_symbol: "API · Drift · RPS" },
    SpellVisualTheme { spell_id: "s19", floor_id: 4, color: "#2dd4bf", bg_color: "#092424", anim_style: "quantize", rune_symbol: "FP32 → INT8 · KB" },
    SpellVisualTheme { spell_id: "s20", floor_id: 4, color: "#fb923c", bg_color: "#281508", anim_style: "edge", rune_symbol: "Σ t_tahap ≤ batas" },
    SpellVisualTheme { spell_id: "s21", floor_id: 5, color: "#f43f5e", bg_color: "#280a14", anim_style: "code", rune_symbol: "git commit · pytest" },
    SpellVisualTheme { spell_id: "s22", floor_id: 5, color: "#10b981", bg_color: "#06241b", anim_style: "prisms", rune_symbol: "w_k = N/(K·N_k)" },
    SpellVisualTheme { spell_id: "s23", floor_id: 5, color: "#f97316", bg_color: "#291205", anim_style: "lightning", rune_symbol: "ADC 2^N · MQTT QoS" },
    SpellVisualTheme { spell_id: "s24", floor_id: 5, color: "#c084fc", bg_color: "#1f0d2e", anim_style: "grid", rune_symbol: "Δ% Baseline · Seed" },
    SpellVisualTheme { spell_id: "s25", floor_id: 5, color: "#eab308", bg_color: "#261d05", anim_style: "forge", rune_symbol: "P(A∩B) = P(A)·P(B)" },
    SpellVisualTheme { spell_id: "s26", floor_id: 5, color: "#ec4899", bg_color: "#290a1d", anim_style: "beam", rune_symbol: "10m Pitch · Q&A" },
    SpellVisualTheme { spell_id: "s27", floor_id: 5, color: "#38bdf8", bg_color: "#082032", anim_style: "schedule", rune_symbol: "MVP · Jam × (1−buf)" },
    SpellVisualTheme { spell_id: "s28", floor_id: 5, color: "#84cc16", bg_color: "#162407", anim_style: "constellation", rune_symbol: "TLS · OWASP IoT" },
];

/// Tema visual satu mantra berdasarkan id (`s1`–`s28`).
pub fn spell_visual(spell_id: &str) -> Option<&'static SpellVisualTheme> {
    SPELL_VISUALS.iter().find(|v| v.spell_id == spell_id)
}

// ============================================================================
// MINI-LAB
// ============================================================================

/// Hasil mini-lab neuron (Topik 12): z dibulatkan 3 desimal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NeuronMiniLabResult {
    pub z: f64,
    pub relu: f64,
    pub sigmoid: f64,
}

pub fn compute_neuron_mini_lab(x1: f64, x2: f64, w1: f64, w2: f64, b: f64) -> NeuronMiniLabResult {
    let z = ((w1 * x1 + w2 * x2 + b) * 1000.0).round() / 1000.0;
    let relu = z.max(0.0);
    let sigmoid = 1.0 / (1.0 + (-z).exp());
    NeuronMiniLabResult { z, relu, sigmoid }
}

/// Hasil mini-lab softmax (Topik 16): pengurangan max untuk stabilitas.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SoftmaxMiniLabResult {
    pub probabilities: [f64; 3],
    pub sum: f64,
}

pub fn compute_softmax_mini_lab(s1: f64, s2: f64, s3: f64) -> SoftmaxMiniLabResult {
    let max_s = s1.max(s2).max(s3);
    let e1 = (s1 - max_s).exp();
    let e2 = (s2 - max_s).exp();
    let e3 = (s3 - max_s).exp();
    let denom = e1 + e2 + e3;
    let p1 = e1 / denom;
    let p2 = e2 / denom;
    let p3 = e3 / denom;
    SoftmaxMiniLabResult {
        probabilities: [p1, p2, p3],
        sum: p1 + p2 + p3,
    }
}

/// Jenis data mini-lab quant (Topik 19).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuantDataType {
    Float32,
    Int8,
}

/// Hasil mini-lab quant: 1 KB = 1000 byte; float32 = 4 byte, int8 = 1 byte.
#[derive(Debug, Clone, PartialEq)]
pub struct QuantMiniLabResult {
    pub bytes_per_parameter: u32,
    pub model_size_bytes: f64,
    pub model_size_kb: f64,
    pub fits_in_flash: bool,
    pub fits_in_ram: bool,
    pub flash_status: &'static str,
    pub ram_status: &'static str,
}

pub fn compute_quant_mini_lab(
    parameter_count: f64,
    data_type: QuantDataType,
    flash_kb: f64,
    ram_kb: f64,
    arena_kb: f64,
) -> QuantMiniLabResult {
    let bytes_per_parameter: u32 = match data_type {
        QuantDataType::Float32 => 4,
        QuantDataType::Int8 => 1,
    };
    let model_size_bytes = parameter_count * bytes_per_parameter as f64;
    let model_size_kb = model_size_bytes / 1000.0;
    let fits_in_flash = model_size_kb <= flash_kb;
    let fits_in_ram = arena_kb <= ram_kb;
    QuantMiniLabResult {
        bytes_per_parameter,
        model_size_bytes,
        model_size_kb,
        fits_in_flash,
        fits_in_ram,
        flash_status: if fits_in_flash {
            "Muat di flash"
        } else {
            "Tidak muat di flash"
        },
        ram_status: if fits_in_ram {
            "Muat di RAM"
        } else {
            "Tidak muat di RAM"
        },
    }
}

/// Hasil mini-lab latensi (Topik 20).
#[derive(Debug, Clone, PartialEq)]
pub struct LatencyMiniLabResult {
    pub total_ms: f64,
    pub limit_ms: f64,
    pub within_budget: bool,
    pub delta_ms: f64,
    pub status_text: String,
}

pub fn compute_latency_mini_lab(
    akuisisi_ms: f64,
    prapemrosesan_ms: f64,
    inferensi_ms: f64,
    pengiriman_ms: f64,
    limit_ms: f64,
) -> LatencyMiniLabResult {
    let total_ms = akuisisi_ms + prapemrosesan_ms + inferensi_ms + pengiriman_ms;
    let within_budget = total_ms <= limit_ms;
    let delta_ms = (limit_ms - total_ms).abs();
    let status_text = if within_budget {
        format!("Sisa {delta_ms} ms")
    } else {
        format!("Melebihi {delta_ms} ms")
    };
    LatencyMiniLabResult {
        total_ms,
        limit_ms,
        within_budget,
        delta_ms,
        status_text,
    }
}

// ============================================================================
// PENCARIAN TOPIK & MANTRA
// ============================================================================

/// Nomor topik dari id roadmap `"topic-N"` (1-28); `None` bila bukan format itu.
pub fn get_numeric_topic_id(topic_id: &str) -> Option<u32> {
    let num_str = topic_id.strip_prefix("topic-")?;
    if num_str.is_empty() || !num_str.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let num: u32 = num_str.parse().ok()?;
    if (1..=28).contains(&num) {
        Some(num)
    } else {
        None
    }
}

/// Id roadmap `"topic-N"` dari nomor topik.
pub fn get_roadmap_topic_id(numeric_topic_id: u32) -> String {
    format!("topic-{numeric_topic_id}")
}

/// Lantai untuk nomor topik: 1-6 → 1, 7-11 → 2, 12-17 → 3, 18-20 → 4,
/// 21-28 → 5, selain itu 1.
pub fn get_floor_for_topic_number(numeric_topic_id: u32) -> u32 {
    match numeric_topic_id {
        1..=6 => 1,
        7..=11 => 2,
        12..=17 => 3,
        18..=20 => 4,
        21..=28 => 5,
        _ => 1,
    }
}

/// Konten satu lantai spellbook (1-5).
pub fn get_floor_content(floor_id: u32) -> Option<&'static FloorContentFile> {
    floor_content(floor_id)
}

/// Konten topik beserta lantai tempatnya.
pub struct TopicContentWithFloor {
    pub topic: &'static TopicContent,
    pub floor_id: u32,
}

pub fn get_topic_content(topic_id: &str) -> Option<TopicContentWithFloor> {
    let num_id = get_numeric_topic_id(topic_id)?;
    for floor_id in 1..=5u32 {
        if let Some(floor_data) = floor_content(floor_id) {
            if let Some(found) = floor_data.topics.iter().find(|t| t.topic_id == num_id) {
                return Some(TopicContentWithFloor {
                    topic: found,
                    floor_id,
                });
            }
        }
    }
    None
}

/// Alias `get_topic_content` (kompatibilitas v2).
pub fn get_floor1_topic_content(topic_id: &str) -> Option<TopicContentWithFloor> {
    get_topic_content(topic_id)
}

/// Mantra yang diperkaya info topik/lantai.
#[derive(Debug, Clone, PartialEq)]
pub struct EnrichedSpell {
    pub id: String,
    pub name: String,
    pub formulas: Vec<String>,
    pub topic_id: u32,
    pub floor_id: u32,
    pub roadmap_topic_id: String,
    /// Paragraf pertama scroll topik (atau "" bila kosong).
    pub summary: String,
}

pub fn get_spells_by_floor(floor_id: u32) -> Vec<EnrichedSpell> {
    let Some(floor_data) = floor_content(floor_id) else {
        return Vec::new();
    };
    floor_data
        .topics
        .iter()
        .map(|t| EnrichedSpell {
            id: t.spell.id.clone(),
            name: t.spell.name.clone(),
            formulas: t.spell.formulas.clone(),
            topic_id: t.topic_id,
            floor_id,
            roadmap_topic_id: get_roadmap_topic_id(t.topic_id),
            summary: t.scroll.paragraphs.first().cloned().unwrap_or_default(),
        })
        .collect()
}

/// Semua 28 mantra Lantai 1-5.
pub fn get_all_spells() -> Vec<EnrichedSpell> {
    (1..=5u32).flat_map(get_spells_by_floor).collect()
}

/// Alias `get_spells_by_floor(1)`.
pub fn get_all_floor1_spells() -> Vec<EnrichedSpell> {
    get_spells_by_floor(1)
}

/// SpellId efektif fase boss: override varian menang atas spellId fase.
pub fn get_effective_boss_spell_id(
    phase_data: &BossPhase,
    variant_data: Option<&BossVariant>,
) -> String {
    if let Some(v) = variant_data {
        if let Some(s) = &v.spell_id {
            return s.clone();
        }
    }
    phase_data.spell_id.clone()
}

// ============================================================================
// DECK PERTEMPURAN & DOJO
// ============================================================================

/// Statistik riwayat satu soal (untuk bobot Dojo/gema).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct QuestionStat {
    pub seen: u32,
    pub wrong: u32,
    pub correct_streak: u32,
}

/// Satu opsi jawaban yang sudah diacak (kebenaran ikut identitas opsi).
#[derive(Debug, Clone, PartialEq)]
pub struct ShuffledOption {
    pub option_id: String,
    pub original_index: usize,
    pub text: String,
    pub is_correct: bool,
}

/// Satu soal siap tempur.
#[derive(Debug, Clone, PartialEq)]
pub struct PreparedBattleQuestion {
    pub id: String,
    pub topic_id: u32,
    pub floor_id: u32,
    pub roadmap_topic_id: String,
    pub spell_id: String,
    pub spell_name: String,
    pub difficulty: Difficulty,
    pub question: String,
    pub explanation: String,
    pub is_echo: bool,
    pub shuffled_options: Vec<ShuffledOption>,
}

/// Mengacak opsi jawaban sambil menjaga kebenaran lewat identitas opsi.
pub fn shuffle_options_by_identity(
    options: &[String],
    correct_index: usize,
    rng: &mut impl Rng,
) -> Vec<ShuffledOption> {
    let mut mapped: Vec<ShuffledOption> = options
        .iter()
        .enumerate()
        .map(|(idx, text)| ShuffledOption {
            option_id: format!("opt-{idx}"),
            original_index: idx,
            text: text.clone(),
            is_correct: idx == correct_index,
        })
        .collect();
    crate::rng::shuffle(&mut mapped, rng);
    mapped
}

fn to_prepared_question(
    topic: &TopicContent,
    floor_id: u32,
    q: &QuizQuestion,
    is_echo: bool,
    rng: &mut impl Rng,
) -> PreparedBattleQuestion {
    PreparedBattleQuestion {
        id: q.id.clone(),
        topic_id: topic.topic_id,
        floor_id,
        roadmap_topic_id: get_roadmap_topic_id(topic.topic_id),
        spell_id: topic.spell.id.clone(),
        spell_name: topic.spell.name.clone(),
        difficulty: q.difficulty,
        question: q.question.clone(),
        explanation: q.explanation.clone(),
        is_echo,
        shuffled_options: shuffle_options_by_identity(&q.options, q.correct_index, rng),
    }
}

/// Bobot Dojo/gema: `max(1, 1 + 2*salah - streakBenar)`.
pub fn calculate_question_weight(stat: Option<&QuestionStat>) -> i64 {
    let wrong = stat.map(|s| s.wrong as i64).unwrap_or(0);
    let streak = stat.map(|s| s.correct_streak as i64).unwrap_or(0);
    (1 + 2 * wrong - streak).max(1)
}

/// Memperbarui statistik soal setelah satu jawaban.
pub fn update_question_stat(prev: Option<&QuestionStat>, is_correct: bool) -> QuestionStat {
    let seen = prev.map(|s| s.seen).unwrap_or(0) + 1;
    if is_correct {
        QuestionStat {
            seen,
            wrong: prev.map(|s| s.wrong).unwrap_or(0),
            correct_streak: prev.map(|s| s.correct_streak).unwrap_or(0) + 1,
        }
    } else {
        QuestionStat {
            seen,
            wrong: prev.map(|s| s.wrong).unwrap_or(0) + 1,
            correct_streak: 0,
        }
    }
}

fn pick_weighted_echo_question<'a>(
    topics: &[(&'a TopicContent, u32)],
    question_stats: &BTreeMap<String, QuestionStat>,
    rng: &mut impl Rng,
) -> Option<(&'a TopicContent, u32, &'a QuizQuestion)> {
    let mut pool: Vec<(&TopicContent, u32, &QuizQuestion, i64)> = Vec::new();
    for (topic, floor_id) in topics {
        for q in &topic.quiz {
            let weight = calculate_question_weight(question_stats.get(&q.id));
            pool.push((topic, *floor_id, q, weight));
        }
    }
    if pool.is_empty() {
        return None;
    }
    let total_weight: i64 = pool.iter().map(|p| p.3).sum();
    let mut roll = rng.next_f64() * total_weight as f64;
    for item in &pool {
        roll -= item.3 as f64;
        if roll <= 0.0 {
            return Some((item.0, item.1, item.2));
        }
    }
    let last = pool.last().unwrap();
    Some((last.0, last.1, last.2))
}

/// Membangun deck pertempuran satu ruangan: 5 soal topik (diacak) + 1 soal
/// gema ("Gema Masa Lalu") bila ada topik yang sudah dikalahkan:
/// - Lantai 1: dari topik Lantai 1 yang sudah dikalahkan.
/// - Lantai 2-4: 60% lantai sebelumnya / 40% lantai saat ini (fallback ke
///   pool mana pun yang tersedia).
/// - Lantai 5: dari SEMUA topik yang sudah dikalahkan, lantai mana pun.
pub fn build_battle_deck(
    topic_id: &str,
    completed_room_ids: &[String],
    question_stats: &BTreeMap<String, QuestionStat>,
    rng: &mut impl Rng,
) -> Vec<PreparedBattleQuestion> {
    let Some(topic_content) = get_topic_content(topic_id) else {
        return Vec::new();
    };
    let current_floor_id = topic_content.floor_id;
    let current_roadmap_id = get_roadmap_topic_id(topic_content.topic.topic_id);

    let mut base_questions: Vec<PreparedBattleQuestion> = topic_content
        .topic
        .quiz
        .iter()
        .map(|q| to_prepared_question(topic_content.topic, current_floor_id, q, false, rng))
        .collect();
    crate::rng::shuffle(&mut base_questions, rng);

    let mut earlier_floor_topics: Vec<(&TopicContent, u32)> = Vec::new();
    let mut current_floor_topics: Vec<(&TopicContent, u32)> = Vec::new();
    let mut all_defeated_topics: Vec<(&TopicContent, u32)> = Vec::new();

    for floor_id in 1..=5u32 {
        let Some(f_content) = floor_content(floor_id) else {
            continue;
        };
        for t in &f_content.topics {
            let r_id = get_roadmap_topic_id(t.topic_id);
            if r_id == current_roadmap_id || !completed_room_ids.iter().any(|c| c == &r_id) {
                continue;
            }
            all_defeated_topics.push((t, floor_id));
            if floor_id < current_floor_id {
                earlier_floor_topics.push((t, floor_id));
            } else if floor_id == current_floor_id {
                current_floor_topics.push((t, floor_id));
            }
        }
    }

    let chosen_pool: Vec<(&TopicContent, u32)> = if current_floor_id == 5 {
        all_defeated_topics
    } else if current_floor_id >= 2 {
        if !earlier_floor_topics.is_empty() && !current_floor_topics.is_empty() {
            if rng.next_f64() < 0.6 {
                earlier_floor_topics
            } else {
                current_floor_topics
            }
        } else if !earlier_floor_topics.is_empty() {
            earlier_floor_topics
        } else {
            current_floor_topics
        }
    } else {
        current_floor_topics
    };

    if chosen_pool.is_empty() {
        return base_questions;
    }
    let Some((picked_topic, picked_floor, picked_q)) =
        pick_weighted_echo_question(&chosen_pool, question_stats, rng)
    else {
        return base_questions;
    };
    let echo_question =
        to_prepared_question(picked_topic, picked_floor, picked_q, true, rng);
    let insert_idx = (rng.next_f64() * (base_questions.len() + 1) as f64).floor() as usize;
    let insert_idx = insert_idx.min(base_questions.len());
    base_questions.insert(insert_idx, echo_question);
    base_questions
}

/// Alias `build_battle_deck` (kompatibilitas v2).
pub fn build_floor1_battle_deck(
    topic_id: &str,
    completed_room_ids: &[String],
    question_stats: &BTreeMap<String, QuestionStat>,
    rng: &mut impl Rng,
) -> Vec<PreparedBattleQuestion> {
    build_battle_deck(topic_id, completed_room_ids, question_stats, rng)
}

/// Hasil evaluasi pertempuran ruangan.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BattleEvaluation {
    pub passed_quiz: bool,
    pub score_percent: i64,
}

/// Lulus = minimal 4 dari 5 soal topik benar; soal gema tidak pernah
/// memengaruhi lulus/gagal.
pub fn evaluate_floor1_battle(
    topic_correct_count: i64,
    topic_total_count: i64,
) -> BattleEvaluation {
    let safe_total = topic_total_count.max(1);
    let score_percent = ((topic_correct_count as f64 / safe_total as f64) * 100.0).round() as i64;
    BattleEvaluation {
        passed_quiz: topic_correct_count >= 4,
        score_percent,
    }
}

/// Bonus crit ruangan: +10% XP ruangan, sekali per ruangan.
pub fn calculate_room_crit_bonus_xp(base_room_xp: i64, crit_bonus_already_claimed: bool) -> i64 {
    if crit_bonus_already_claimed {
        return 0;
    }
    (base_room_xp as f64 * CRIT_ROOM_BONUS_RATE).round() as i64
}

/// Satu soal Dojo beserta bobotnya.
#[derive(Debug, Clone, PartialEq)]
pub struct DojoQuestion {
    pub question: PreparedBattleQuestion,
    pub weight: i64,
}

/// Memilih hingga `max_questions` soal dari topik Lantai 1-5 yang sudah
/// dikalahkan, berbobot riwayat, tanpa pengulangan.
pub fn build_dojo_question_set(
    completed_room_ids: &[String],
    question_stats: &BTreeMap<String, QuestionStat>,
    max_questions: usize,
    rng: &mut impl Rng,
) -> Vec<DojoQuestion> {
    let mut pool: Vec<(&TopicContent, u32, &QuizQuestion, i64)> = Vec::new();
    for floor_id in 1..=5u32 {
        let Some(f_content) = floor_content(floor_id) else {
            continue;
        };
        for t in &f_content.topics {
            if !completed_room_ids
                .iter()
                .any(|c| c == &get_roadmap_topic_id(t.topic_id))
            {
                continue;
            }
            for q in &t.quiz {
                let weight = calculate_question_weight(question_stats.get(&q.id));
                pool.push((t, floor_id, q, weight));
            }
        }
    }

    let mut selected: Vec<DojoQuestion> = Vec::new();
    let mut remaining = pool;
    let target_count = max_questions.min(remaining.len());

    for _ in 0..target_count {
        let total_weight: i64 = remaining.iter().map(|p| p.3).sum();
        let mut roll = rng.next_f64() * total_weight as f64;
        let mut chosen_idx = 0usize;
        for (i, item) in remaining.iter().enumerate() {
            roll -= item.3 as f64;
            if roll <= 0.0 {
                chosen_idx = i;
                break;
            }
        }
        let picked = remaining.remove(chosen_idx);
        let question = to_prepared_question(picked.0, picked.1, picked.2, false, rng);
        selected.push(DojoQuestion {
            question,
            weight: picked.3,
        });
    }
    selected
}

// ============================================================================
// ATURAN BOSS
// ============================================================================

/// Varian boss berikutnya: A (0) dulu, lalu B (1), lalu bergantian.
pub fn get_next_boss_variant_index(previous_variant_index: Option<u8>) -> u8 {
    match previous_variant_index {
        None => 0,
        Some(0) => 1,
        Some(_) => 0,
    }
}

/// Pulih setelah pingsan: boss merebut kembali satu segmen (fase terakhir
/// yang sudah dibersihkan dihapus); tidak pernah di bawah 0.
pub fn recover_boss_after_faint(cleared_phases: &[u32]) -> Vec<u32> {
    if cleared_phases.is_empty() {
        return Vec::new();
    }
    cleared_phases[..cleared_phases.len() - 1].to_vec()
}

// ============================================================================
// LANTAI 6 — MENARA MASTER
// ============================================================================

/// Meta relik Menara Master (diturunkan dari quest Lantai 6).
#[derive(Debug, Clone, PartialEq)]
pub struct TowerRelicMeta {
    pub id: String,
    pub quest_id: String,
    pub name: String,
    pub source: String,
    pub icon_type: String,
    pub description: String,
}

/// Relik Menara Master: satu per quest, `id = "relic-{questId}"`.
pub fn tower_relics() -> Vec<TowerRelicMeta> {
    floor6_content()
        .quests
        .iter()
        .enumerate()
        .map(|(idx, q)| TowerRelicMeta {
            id: format!("relic-{}", q.id),
            quest_id: q.id.clone(),
            name: q.relic.clone(),
            source: format!("Menara Master — Lantai Menara {}: {}", idx + 1, q.title),
            icon_type: format!("tower-{}", q.id),
            description: q.summary.clone(),
        })
        .collect()
}

/// Id relik untuk satu quest Menara.
pub fn get_tower_relic_id(quest_id: &str) -> String {
    format!("relic-{quest_id}")
}

/// Quest Menara berdasarkan id (`m1`–`m7`).
pub fn get_tower_quest_by_id(quest_id: &str) -> Option<&'static TowerQuestContent> {
    floor6_content().quests.iter().find(|q| q.id == quest_id)
}

/// Kemajuan langkah satu quest: (dicentang, total).
pub fn get_tower_quest_step_progress(
    quest_id: &str,
    tower_steps: &BTreeMap<String, BTreeMap<String, bool>>,
) -> (usize, usize) {
    let Some(quest) = get_tower_quest_by_id(quest_id) else {
        return (0, 0);
    };
    let step_map = tower_steps.get(quest_id);
    let ticked = quest
        .steps
        .iter()
        .filter(|s| step_map.and_then(|m| m.get(&s.id)).copied().unwrap_or(false))
        .count();
    (ticked, quest.steps.len())
}

/// Status quest Menara.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TowerQuestStatus {
    BelumDimulai,
    Berjalan,
    Selesai,
}

impl TowerQuestStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TowerQuestStatus::BelumDimulai => "Belum dimulai",
            TowerQuestStatus::Berjalan => "Berjalan",
            TowerQuestStatus::Selesai => "Selesai",
        }
    }
}

pub fn get_tower_quest_status(
    quest_id: &str,
    tower_steps: &BTreeMap<String, BTreeMap<String, bool>>,
    tower_completed: &[String],
) -> TowerQuestStatus {
    if tower_completed.iter().any(|c| c == quest_id) {
        return TowerQuestStatus::Selesai;
    }
    let (ticked, _) = get_tower_quest_step_progress(quest_id, tower_steps);
    if ticked > 0 {
        TowerQuestStatus::Berjalan
    } else {
        TowerQuestStatus::BelumDimulai
    }
}

/// Mencentang langkah `m6s3` ("Pilih satu spesialisasi...") hanya boleh
/// setelah spesialisasi dipilih.
pub fn can_tick_tower_step(
    quest_id: &str,
    step_id: &str,
    specialization: Option<&str>,
) -> bool {
    if quest_id == "m6" && step_id == "m6s3" {
        return specialization.map(|s| !s.trim().is_empty()).unwrap_or(false);
    }
    true
}

/// Quest Menara bisa ditandai selesai bila: belum selesai, semua langkah
/// dicentang, dan (untuk m6) spesialisasi sudah dipilih.
pub fn can_complete_tower_quest(
    quest_id: &str,
    tower_steps: &BTreeMap<String, BTreeMap<String, bool>>,
    tower_completed: &[String],
    specialization: Option<&str>,
) -> bool {
    if tower_completed.iter().any(|c| c == quest_id) {
        return false;
    }
    let Some(quest) = get_tower_quest_by_id(quest_id) else {
        return false;
    };
    if quest_id == "m6" && !specialization.map(|s| !s.trim().is_empty()).unwrap_or(false) {
        return false;
    }
    let step_map = tower_steps.get(quest_id);
    quest
        .steps
        .iter()
        .all(|s| step_map.and_then(|m| m.get(&s.id)).copied().unwrap_or(false))
}

/// Cermin Puncak terbuka bila ke-7 quest Menara selesai.
pub fn can_unlock_cermin_puncak(tower_completed: &[String]) -> bool {
    floor6_content()
        .quests
        .iter()
        .all(|q| tower_completed.iter().any(|c| c == &q.id))
}

/// Semua 5 prompt refleksi sudah dijawab (tidak kosong).
pub fn are_all_reflections_answered(reflections: &BTreeMap<String, String>) -> bool {
    floor6_content()
        .guardian
        .reflection_prompts
        .iter()
        .all(|p| {
            reflections
                .get(&p.id)
                .map(|v| !v.trim().is_empty())
                .unwrap_or(false)
        })
}

/// Masukan teks ringkasan Piagam ("Salin ringkasan").
#[derive(Debug, Clone, PartialEq)]
pub struct CertificateSummaryInput {
    pub player_name: String,
    pub level: i64,
    pub level_title: String,
    pub rooms_defeated: usize,
    pub total_rooms: usize,
    pub bosses_defeated: usize,
    pub total_bosses: usize,
    pub relics_count: usize,
    pub total_relics: usize,
    pub tower_quests_completed: usize,
    pub total_tower_quests: usize,
    pub specialization: Option<String>,
    pub issued_date_formatted: String,
}

/// Teks ringkasan Piagam; memakai judul, honestNote, dan closing persis
/// dari `floor6.content.json` (judul di-UPPERCASE-kan).
pub fn format_certificate_summary_text(input: &CertificateSummaryInput) -> String {
    let cert = &floor6_content().certificate;
    let spec_text = match &input.specialization {
        Some(s) if !s.trim().is_empty() => s.trim().to_string(),
        _ => "Belum dipilih".to_string(),
    };
    [
        format!("=== {} ===", cert.title.to_uppercase()),
        format!("Nama Petualang: {}", input.player_name),
        format!(
            "Level & Gelar: Level {} — {}",
            input.level, input.level_title
        ),
        format!(
            "Ruang/Quest Dikalahkan: {}/{}",
            input.rooms_defeated, input.total_rooms
        ),
        format!(
            "Boss Dikalahkan: {}/{}",
            input.bosses_defeated, input.total_bosses
        ),
        format!("Jumlah Relik: {}/{}", input.relics_count, input.total_relics),
        format!(
            "Quest Menara Selesai: {}/{}",
            input.tower_quests_completed, input.total_tower_quests
        ),
        format!("Spesialisasi Pilihan: {spec_text}"),
        format!("Tanggal Terbit: {}", input.issued_date_formatted),
        String::new(),
        cert.honest_note.clone(),
        cert.closing.clone(),
    ]
    .join("\n")
}

// ============================================================================
// DIALOG MONSTER & TOPIK TERLEMAH
// ============================================================================

/// Dialog singkat monster lantai per topik (maks 2 kalimat).
pub fn get_monster_dialogue(topic_id: &str, fallback_flavor: &str) -> String {
    let dialogue: Option<&str> = match topic_id {
        "topic-1" => Some("Hahaha! Banyak petualang tersesat saat melihat rumus panjang, padahal rahasianya hanya menyederhanakan pecahan dan membaca kemiringan grafik!"),
        "topic-2" => Some("Jangan tertipu oleh angka rata-rata semata! Buktikan dulu apakah kamu tahu cara memperbarui peluang lewat Teorema Bayes saat bukti baru muncul!"),
        "topic-3" => Some("Nilai p-value kecil sering disalahartikan oleh pemula yang terburu-buru! Beranikah kamu menjelaskan maknanya tanpa terjebak ilusi kebetulan?"),
        "topic-4" => Some("Setiap baris dan kolom di tubuhku adalah ruang vektor! Jika kamu tak paham mengapa matriks dikalikan, langkahmu di dunia ML akan terhenti di sini!"),
        "topic-5" => Some("Rasakan curamnya lereng fungsi ini! Tanpa arah gradien dan aturan rantai, kamu hanya akan berputar-putar di puncak bukit error!"),
        "topic-6" => Some("Tabel dataku penuh nilai kosong dan angka berantakan! Bersihkan aku dengan NumPy dan pandas, lalu tunjukkan wujud asliku lewat grafik matplotlib!"),
        "topic-7" => Some("Akar-akarku membelah tanah menjadi kawasan latih, validasi, dan uji yang tak boleh saling bocor! Beranikah kau melangkah tanpa mengintip masa depan?"),
        "topic-8" => Some("Di hutan yang timpang ini, akurasi semu hanyalah fatamorgana daun kering! Tunjukkan keseimbangan presisi dan recall-mu sebelum melintas!"),
        "topic-9" => Some("Setiap dahan di tubuhku bercabang mengikuti penurunan impuritas Gini! Tebaslah ambang batas keputusanku jika kau paham rahasia kurva sigmoid!"),
        "topic-10" => Some("Kayu mentah takkan menjadi perisai kokoh tanpa pembersihan dan penyetaraan skala! Hanya fitur yang diolah tanpa kebocoran yang sanggup menembus kulitku!"),
        "topic-11" => Some("Tanpa label penunjuk jalan, gugus-gugus spora di rimba ini berkumpul menurut jarak terdekat! Temukan pusat klaster dan sumbu variansi utamaku!"),
        "topic-12" => Some("Denyut sinapsisku memadukan bobot dan bias sebelum membangkitkan gerbang aktivasi non-linear! Tanpa aktivasi yang tepat, lapisanku hanyalah bayangan garis lurus biasa!"),
        "topic-13" => Some("Aliran gradien di tubuhku tak pernah lupa dijumlahkan jika kau lalai mengosongkannya! Tunjukkan ritme lima langkah latihan PyTorch sebelum melangkah lebih dalam!"),
        "topic-14" => Some("Mataku memindai setiap petak piksel dengan filter lokal yang berbagi bobot di seluruh ruang! Bisakah kau menghitung ukuran peta fiturku setelah melintasi stride dan pooling?"),
        "topic-15" => Some("Ingatanku mengalir dari langkah waktu ke langkah waktu melalui keadaan tersembunyi yang berantai! Waspadalah terhadap gradien yang meredup di lorong urutan panjang ini!"),
        "topic-16" => Some("Setiap kepalaku menimbang Query dan Key untuk memusatkan perhatian langsung ke bagian urutan terpenting! Bagi skor dot-product dengan akar dimensi agar bobot softmax-ku tidak membeku!"),
        "topic-17" => Some("Hanya penjadwalan laju belajar, pencari AdamW, dan perisai regularisasi yang sanggup menjaga latihanku tetap stabil! Buktikan kau mampu meredam overfitting di kuil terdalam ini!"),
        "topic-18" => Some("Kontainer serverku hanya menerima model yang dikemas rapi beserta langkah prapemrosesannya! Waspadalah terhadap pergeseran distribusi data saat sistemmu berjalan di produksi!"),
        "topic-19" => Some("Sirkuit mikrokontrolerku hanya menyediakan ruang kilobyte yang sempit di flash dan RAM! Padatkan bobot float32-mu menjadi int8 dan pastikan tensor arena muat sebelum melintas!"),
        "topic-20" => Some("Dari pembacaan sensor hingga aksi perangkat, setiap milidetik dihitung dalam anggaran latensi total! Bangunlah pipa ujung-ke-ujung yang tangguh dan temukan penghambat utamanya!"),
        "topic-21" => Some("Satu commit raksasa tanpa pesan jelas atau kunci rahasia yang bocor di riwayat Git akan mengacaukan proyekmu! Tunjukkan cabang yang rapi dan barisan tes otomatis yang lulus!"),
        "topic-22" => Some("Data sensor dunia nyata penuh nilai hilang, derau, dan kelas yang timpang! Selidiki penyebab kejanggalan data dan catat kualitasnya sebelum kau berani melatih model!"),
        "topic-23" => Some("Tegangan ADC, kalibrasi dua titik, dan tingkat QoS MQTT menentukan apakah perangkatmu bertahan 24 jam! Hitung daya bateraimu dengan cermat sebelum sirkuit ini padam!"),
        "topic-24" => Some("Skor tinggi tanpa pembanding hanyalah ilusi jika kalah oleh baseline sederhana! Buktikan peningkatan relatif modelmu dan catat seed eksperimen agar hasilnya jujur serta dapat diulang!"),
        "topic-25" => Some("Semakin banyak komponen saat demo, semakin besar peluang ada yang gagal di panggung! Siapkan log yang jelas, langkah reproduksi minimal, serta rekaman cadangan sebelum hari-H!"),
        "topic-26" => Some("Waktu sepuluh menit di panggung akan habis jika kau tenggelam dalam tabel mentah! Sampaikan satu pesan per slide dan jawab pertanyaan sulit dengan jujur beserta data analisis kesalahan!"),
        "topic-27" => Some("Fitur tambahan yang terus menyelinap akan menenggelamkan proyek solomu sebelum selesai! Kunci rencana satu halaman dengan batas lingkup tertulis dan fokuslah menuntaskan MVP!"),
        "topic-28" => Some("Kata sandi bawaan dan broker terbuka adalah celah empuk bagi penyusup perangkat IoT! Terapkan enkripsi, minimisasi data pengguna, dan siapkan jawaban keamanan untuk setiap proyekmu!"),
        _ => None,
    };
    dialogue.unwrap_or(fallback_flavor).to_string()
}

/// Ringkasan satu topik terlemah untuk seksi "Kelemahan" Dashboard.
#[derive(Debug, Clone, PartialEq)]
pub struct WeakTopicSummary {
    pub topic_id: String,
    pub number: u32,
    pub floor_id: u32,
    pub name: String,
    pub best_score_percent: i64,
    pub needs_review: bool,
}

/// Daftar topik terlemah (skor terbaik terendah dulu, lalu nomor topik),
/// maksimal `max_items` (bawaan 5).
pub fn get_weakest_topics(
    quiz_best_scores: &BTreeMap<String, i64>,
    max_items: usize,
) -> Vec<WeakTopicSummary> {
    let mut entries: Vec<WeakTopicSummary> = Vec::new();
    for (topic_id, best_score) in quiz_best_scores {
        let Some(topic) = topic_by_id(topic_id) else {
            continue;
        };
        entries.push(WeakTopicSummary {
            topic_id: topic_id.clone(),
            number: topic.number,
            floor_id: topic.floor_id,
            name: topic.name.clone(),
            best_score_percent: *best_score,
            needs_review: *best_score < 100,
        });
    }
    entries.sort_by(|a, b| {
        a.best_score_percent
            .cmp(&b.best_score_percent)
            .then(a.number.cmp(&b.number))
    });
    entries.truncate(max_items);
    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::ConstRng;

    #[test]
    fn floor_content_verbatim_structures() {
        // Port spellbookEngine.test.ts: struktur verbatim lantai 1-5.
        let f1 = floor_content(1).unwrap();
        assert_eq!(f1.review_status, "needs-human-review");
        assert_eq!(f1.floor, 1);
        assert_eq!(f1.topics.len(), 6);
        for (i, t) in f1.topics.iter().enumerate() {
            assert_eq!(t.topic_id, (i + 1) as u32);
            assert_eq!(t.spell.id, format!("s{}", i + 1));
            assert_eq!(t.quiz.len(), 5);
        }
        assert_eq!(f1.boss.name, "Raja Matematika");
        assert_eq!(f1.boss.phases.len(), 6);
        for p in &f1.boss.phases {
            assert_eq!(p.variants.len(), 2);
        }

        let f2 = floor_content(2).unwrap();
        assert_eq!(f2.review_status, "needs-human-review");
        assert_eq!(f2.floor, 2);
        assert_eq!(f2.topics.len(), 5);
        for (i, t) in f2.topics.iter().enumerate() {
            assert_eq!(t.topic_id, (i + 7) as u32);
            assert_eq!(t.spell.id, format!("s{}", i + 7));
            assert_eq!(t.quiz.len(), 5);
        }
        assert_eq!(f2.boss.name, "Golem Pohon Purba");
        assert_eq!(f2.boss.phases.len(), 6);
        let phase6 = &f2.boss.phases[5];
        assert_eq!(phase6.phase, 6);
        assert_eq!(phase6.title.as_deref(), Some("Gema Lantai 1"));
        assert_eq!(
            get_effective_boss_spell_id(phase6, Some(&phase6.variants[0])),
            "s5"
        );
        assert_eq!(
            get_effective_boss_spell_id(phase6, Some(&phase6.variants[1])),
            "s2"
        );

        let f3 = floor_content(3).unwrap();
        assert_eq!(f3.review_status, "needs-human-review");
        assert_eq!(f3.floor, 3);
        assert_eq!(f3.topics.len(), 6);
        for (i, t) in f3.topics.iter().enumerate() {
            assert_eq!(t.topic_id, (i + 12) as u32);
            assert_eq!(t.spell.id, format!("s{}", i + 12));
            assert_eq!(t.quiz.len(), 5);
        }
        assert_eq!(f3.topics[0].mini_lab.as_deref(), Some("neuron"));
        assert_eq!(f3.topics[4].mini_lab.as_deref(), Some("softmax"));
        assert_eq!(f3.boss.name, "Hydra Berkepala Banyak");
        assert_eq!(f3.boss.phases.len(), 7);
        assert_eq!(get_total_boss_phases(3), 7);
        assert_eq!(get_initial_boss_potions(3), 3);
        assert_eq!(get_initial_boss_potions(2), 2);
        let phase7 = &f3.boss.phases[6];
        assert_eq!(phase7.phase, 7);
        assert_eq!(phase7.title.as_deref(), Some("Gema Lantai 1-2"));
        assert_eq!(
            get_effective_boss_spell_id(phase7, Some(&phase7.variants[0])),
            "s5"
        );
        assert_eq!(
            get_effective_boss_spell_id(phase7, Some(&phase7.variants[1])),
            "s8"
        );

        let f4 = floor_content(4).unwrap();
        assert_eq!(f4.review_status, "needs-human-review");
        assert_eq!(f4.floor, 4);
        assert_eq!(f4.topics.len(), 3);
        for (i, t) in f4.topics.iter().enumerate() {
            assert_eq!(t.topic_id, (i + 18) as u32);
            assert_eq!(t.spell.id, format!("s{}", i + 18));
            assert_eq!(t.quiz.len(), 5);
        }
        assert_eq!(f4.topics[0].mini_lab, None);
        assert_eq!(f4.topics[1].mini_lab.as_deref(), Some("quant"));
        assert_eq!(f4.topics[2].mini_lab.as_deref(), Some("latency"));
        assert_eq!(f4.boss.name, "Mecha-Core");
        assert_eq!(f4.boss.phases.len(), 5);
        assert_eq!(get_total_boss_phases(4), 5);
        assert_eq!(get_initial_boss_potions(4), 2);
        let phase4 = &f4.boss.phases[3];
        assert_eq!(phase4.phase, 4);
        assert_eq!(phase4.title.as_deref(), Some("Gema Lantai 2-3"));
        assert_eq!(
            get_effective_boss_spell_id(phase4, Some(&phase4.variants[0])),
            "s14"
        );
        assert_eq!(
            get_effective_boss_spell_id(phase4, Some(&phase4.variants[1])),
            "s9"
        );
        let phase5 = &f4.boss.phases[4];
        assert_eq!(phase5.phase, 5);
        assert_eq!(phase5.title.as_deref(), Some("Gema Lantai 1 dan 3"));
        assert_eq!(
            get_effective_boss_spell_id(phase5, Some(&phase5.variants[0])),
            "s2"
        );
        assert_eq!(
            get_effective_boss_spell_id(phase5, Some(&phase5.variants[1])),
            "s15"
        );

        let all_spells = get_all_spells();
        assert_eq!(all_spells.len(), 28);
        assert_eq!(get_spells_by_floor(1).len(), 6);
        assert_eq!(get_spells_by_floor(2).len(), 5);
        assert_eq!(get_spells_by_floor(3).len(), 6);
        assert_eq!(get_spells_by_floor(4).len(), 3);
        assert_eq!(get_spells_by_floor(5).len(), 8);
        let mut colors: Vec<&str> = all_spells
            .iter()
            .filter_map(|s| spell_visual(&s.id).map(|v| v.color))
            .collect();
        colors.sort_unstable();
        colors.dedup();
        assert_eq!(colors.len(), 28);

        let f5 = floor_content(5).unwrap();
        assert_eq!(f5.review_status, "needs-human-review");
        assert_eq!(f5.floor, 5);
        assert_eq!(f5.topics.len(), 8);
        let expected_relics = [
            "Kompas Git",
            "Lensa Data",
            "Papan Sirkuit",
            "Neraca Jujur",
            "Tameng Cadangan",
            "Mikrofon Emas",
            "Peta Lingkup",
            "Kunci Perisai",
        ];
        for (i, t) in f5.topics.iter().enumerate() {
            assert_eq!(t.topic_id, (i + 21) as u32);
            assert_eq!(t.spell.id, format!("s{}", i + 21));
            assert_eq!(t.relic.as_deref(), Some(expected_relics[i]));
            assert!(t.evidence_hint.as_ref().is_some_and(|s| !s.is_empty()));
            assert_eq!(t.quiz.len(), 5);
        }
        assert_eq!(f5.boss.name, "Raja Deadline");
        assert_eq!(f5.boss.phases.len(), 8);
        assert_eq!(get_total_boss_phases(5), 8);
        assert_eq!(get_initial_boss_potions(5), 3);
        for (i, p) in f5.boss.phases.iter().enumerate() {
            assert_eq!(p.phase, (i + 1) as u32);
            assert_eq!(p.topic_id, (i + 21) as u32);
            assert_eq!(p.spell_id, format!("s{}", i + 21));
            assert_eq!(p.variants.len(), 2);
        }
    }

    #[test]
    fn mini_lab_math_helpers() {
        // Port spellbookEngine.test.ts: contoh workedExample topik 12 & 16.
        let neuron = compute_neuron_mini_lab(2.0, -1.0, 1.5, 2.0, -0.5);
        assert_eq!(neuron.z, 0.5);
        assert_eq!(neuron.relu, 0.5);
        assert!(neuron.sigmoid > 0.62 && neuron.sigmoid < 0.63);

        let neg = compute_neuron_mini_lab(-2.0, -1.0, 1.0, 1.0, 0.0);
        assert_eq!(neg.z, -3.0);
        assert_eq!(neg.relu, 0.0);

        let sm_equal = compute_softmax_mini_lab(0.0, 0.0, 0.0);
        assert!((sm_equal.sum - 1.0).abs() < 1e-9);
        assert!((sm_equal.probabilities[0] - 1.0 / 3.0).abs() < 1e-6);

        let sm_ex = compute_softmax_mini_lab(2.0, 1.0, 0.0);
        assert!((sm_ex.sum - 1.0).abs() < 1e-9);
        assert!((sm_ex.probabilities[0] - 0.665).abs() < 0.01);
        assert!((sm_ex.probabilities[1] - 0.244).abs() < 0.01);
        assert!((sm_ex.probabilities[2] - 0.09).abs() < 0.01);

        let quant_float =
            compute_quant_mini_lab(50000.0, QuantDataType::Float32, 1000.0, 256.0, 50.0);
        assert_eq!(quant_float.bytes_per_parameter, 4);
        assert_eq!(quant_float.model_size_bytes, 200000.0);
        assert_eq!(quant_float.model_size_kb, 200.0);
        assert!(quant_float.fits_in_flash);
        assert!(quant_float.fits_in_ram);
        assert_eq!(quant_float.flash_status, "Muat di flash");
        assert_eq!(quant_float.ram_status, "Muat di RAM");

        let quant_int8 = compute_quant_mini_lab(50000.0, QuantDataType::Int8, 40.0, 256.0, 300.0);
        assert_eq!(quant_int8.bytes_per_parameter, 1);
        assert_eq!(quant_int8.model_size_kb, 50.0);
        assert!(!quant_int8.fits_in_flash);
        assert!(!quant_int8.fits_in_ram);

        let lat_within = compute_latency_mini_lab(15.0, 10.0, 35.0, 20.0, 100.0);
        assert_eq!(lat_within.total_ms, 80.0);
        assert!(lat_within.within_budget);
        assert_eq!(lat_within.status_text, "Sisa 20 ms");

        let lat_over = compute_latency_mini_lab(40.0, 30.0, 50.0, 20.0, 100.0);
        assert_eq!(lat_over.total_ms, 140.0);
        assert!(!lat_over.within_budget);
        assert_eq!(lat_over.status_text, "Melebihi 40 ms");
    }

    #[test]
    fn shuffle_options_by_identity_keeps_correctness() {
        let options = vec![
            "1/3".to_string(),
            "1/4".to_string(),
            "1/6".to_string(),
            "3/4".to_string(),
        ];
        let shuffled = shuffle_options_by_identity(&options, 1, &mut ConstRng(0.01));
        assert_eq!(shuffled.len(), 4);
        let correct: Vec<&ShuffledOption> =
            shuffled.iter().filter(|o| o.is_correct).collect();
        assert_eq!(correct.len(), 1);
        assert_eq!(correct[0].text, "1/4");
        assert_eq!(correct[0].original_index, 1);
    }

    #[test]
    fn battle_deck_echo_rules_floors_1_4_5() {
        // Port spellbookEngine.test.ts (rng deterministik di Rust).
        let empty_stats: BTreeMap<String, QuestionStat> = BTreeMap::new();

        let deck_no_defeated =
            build_floor1_battle_deck("topic-1", &[], &empty_stats, &mut ConstRng(0.5));
        assert_eq!(deck_no_defeated.len(), 5);
        assert_eq!(deck_no_defeated.iter().filter(|q| q.is_echo).count(), 0);

        let deck_with_echo = build_floor1_battle_deck(
            "topic-2",
            &["topic-1".to_string()],
            &empty_stats,
            &mut ConstRng(0.5),
        );
        assert_eq!(deck_with_echo.len(), 6);
        let echoes: Vec<&PreparedBattleQuestion> =
            deck_with_echo.iter().filter(|q| q.is_echo).collect();
        assert_eq!(echoes.len(), 1);
        assert_eq!(echoes[0].roadmap_topic_id, "topic-1");

        // Lantai 4, rng 0.3 (< 0.6): gema dari lantai sebelumnya (topic-12).
        let deck_f4_earlier = build_battle_deck(
            "topic-19",
            &["topic-12".to_string(), "topic-18".to_string()],
            &empty_stats,
            &mut ConstRng(0.3),
        );
        let echo_earlier = deck_f4_earlier.iter().find(|q| q.is_echo).unwrap();
        assert_eq!(echo_earlier.floor_id, 3);
        assert_eq!(echo_earlier.roadmap_topic_id, "topic-12");

        // Lantai 4, rng 0.85 (>= 0.6): gema dari lantai saat ini (topic-18).
        let deck_f4_current = build_battle_deck(
            "topic-19",
            &["topic-12".to_string(), "topic-18".to_string()],
            &empty_stats,
            &mut ConstRng(0.85),
        );
        let echo_current = deck_f4_current.iter().find(|q| q.is_echo).unwrap();
        assert_eq!(echo_current.floor_id, 4);
        assert_eq!(echo_current.roadmap_topic_id, "topic-18");

        // Lantai 5, save baru: tanpa gema (5 soal).
        let deck_f5_fresh =
            build_battle_deck("topic-21", &[], &empty_stats, &mut ConstRng(0.5));
        assert_eq!(deck_f5_fresh.len(), 5);
        assert_eq!(deck_f5_fresh.iter().filter(|q| q.is_echo).count(), 0);

        // Lantai 5: gema dari SEMUA lantai yang sudah dikalahkan.
        let deck_f5_any = build_battle_deck(
            "topic-22",
            &["topic-21".to_string(), "topic-3".to_string()],
            &empty_stats,
            &mut ConstRng(0.5),
        );
        assert_eq!(deck_f5_any.len(), 6);
        let echo_f5 = deck_f5_any.iter().find(|q| q.is_echo).unwrap();
        assert!(
            echo_f5.roadmap_topic_id == "topic-21" || echo_f5.roadmap_topic_id == "topic-3"
        );
    }

    #[test]
    fn dojo_weight_and_floor4_inclusion() {
        let mut stat = update_question_stat(None, false);
        assert_eq!(stat.wrong, 1);
        assert_eq!(stat.correct_streak, 0);
        assert_eq!(calculate_question_weight(Some(&stat)), 3);

        stat = update_question_stat(Some(&stat), false);
        assert_eq!(calculate_question_weight(Some(&stat)), 5);

        stat = update_question_stat(Some(&stat), true);
        assert_eq!(calculate_question_weight(Some(&stat)), 4);

        let mut stats: BTreeMap<String, QuestionStat> = BTreeMap::new();
        stats.insert("t18q1".to_string(), stat);
        let dojo_set = build_dojo_question_set(
            &[
                "topic-1".to_string(),
                "topic-7".to_string(),
                "topic-12".to_string(),
                "topic-18".to_string(),
            ],
            &stats,
            8,
            &mut ConstRng(0.9),
        );
        assert_eq!(dojo_set.len(), 8);
        assert!(dojo_set.iter().any(|q| q.question.floor_id == 4));
    }

    #[test]
    fn battle_evaluation_crit_and_boss_rules() {
        assert!(evaluate_floor1_battle(4, 5).passed_quiz);
        assert!(!evaluate_floor1_battle(3, 5).passed_quiz);

        assert_eq!(calculate_room_crit_bonus_xp(150, false), 15);
        assert_eq!(calculate_room_crit_bonus_xp(150, true), 0);

        assert_eq!(get_next_boss_variant_index(None), 0);
        assert_eq!(get_next_boss_variant_index(Some(0)), 1);
        assert_eq!(get_next_boss_variant_index(Some(1)), 0);

        assert_eq!(recover_boss_after_faint(&[1, 2, 3]), vec![1, 2]);
        assert!(recover_boss_after_faint(&[]).is_empty());
    }

    #[test]
    fn floor6_tower_rules_and_certificate() {
        let f6 = floor6_content();
        assert_eq!(f6.review_status, "needs-human-review");
        assert_eq!(f6.floor, 6);
        assert!(f6.steps_are_suggestions);
        assert_eq!(f6.quests.len(), 7);
        assert_eq!(f6.specializations.len(), 5);
        assert_eq!(f6.guardian.name, "Cermin Puncak");
        assert_eq!(f6.guardian.reflection_prompts.len(), 5);
        assert_eq!(f6.certificate.title, "Piagam Penyelesai Peta Belajar");
        assert_eq!(tower_relics().len(), 7);

        for q in &f6.quests {
            assert_eq!(q.xp, 150);
            assert!(!q.relic.is_empty());
            assert!(q.steps.len() >= 4);
        }

        assert_eq!(
            get_tower_quest_status("m1", &BTreeMap::new(), &[]),
            TowerQuestStatus::BelumDimulai
        );
        let mut steps_m1: BTreeMap<String, BTreeMap<String, bool>> = BTreeMap::new();
        steps_m1.insert(
            "m1".to_string(),
            [("m1s1".to_string(), true)].into_iter().collect(),
        );
        assert_eq!(
            get_tower_quest_status("m1", &steps_m1, &[]),
            TowerQuestStatus::Berjalan
        );
        assert_eq!(
            get_tower_quest_status("m1", &steps_m1, &["m1".to_string()]),
            TowerQuestStatus::Selesai
        );

        let mut steps2: BTreeMap<String, BTreeMap<String, bool>> = BTreeMap::new();
        steps2.insert(
            "m1".to_string(),
            [
                ("m1s1".to_string(), true),
                ("m1s2".to_string(), true),
            ]
            .into_iter()
            .collect(),
        );
        assert_eq!(get_tower_quest_step_progress("m1", &steps2), (2, 5));

        assert!(!can_tick_tower_step("m6", "m6s3", None));
        assert!(!can_tick_tower_step("m6", "m6s3", Some("")));
        assert!(can_tick_tower_step("m6", "m6s3", Some("TinyML")));
        assert!(can_tick_tower_step("m6", "m6s1", None));

        let mut m6_steps: BTreeMap<String, BTreeMap<String, bool>> = BTreeMap::new();
        m6_steps.insert(
            "m6".to_string(),
            [
                ("m6s1".to_string(), true),
                ("m6s2".to_string(), true),
                ("m6s3".to_string(), true),
                ("m6s4".to_string(), true),
            ]
            .into_iter()
            .collect(),
        );
        assert!(!can_complete_tower_quest("m6", &m6_steps, &[], None));
        assert!(can_complete_tower_quest(
            "m6",
            &m6_steps,
            &[],
            Some("TinyML")
        ));
        assert!(!can_complete_tower_quest(
            "m6",
            &m6_steps,
            &["m6".to_string()],
            Some("TinyML")
        ));

        assert!(!can_unlock_cermin_puncak(&[
            "m1".to_string(),
            "m2".to_string(),
            "m3".to_string(),
            "m4".to_string(),
            "m5".to_string(),
            "m6".to_string(),
        ]));
        assert!(can_unlock_cermin_puncak(&[
            "m1".to_string(),
            "m2".to_string(),
            "m3".to_string(),
            "m4".to_string(),
            "m5".to_string(),
            "m6".to_string(),
            "m7".to_string(),
        ]));

        let mut refl: BTreeMap<String, String> = BTreeMap::new();
        refl.insert("r1".to_string(), "A".to_string());
        refl.insert("r2".to_string(), "B".to_string());
        refl.insert("r3".to_string(), "C".to_string());
        refl.insert("r4".to_string(), "D".to_string());
        assert!(!are_all_reflections_answered(&refl));
        refl.insert("r5".to_string(), "   ".to_string());
        assert!(!are_all_reflections_answered(&refl));
        refl.insert("r5".to_string(), "E".to_string());
        assert!(are_all_reflections_answered(&refl));

        let cert_text = format_certificate_summary_text(&CertificateSummaryInput {
            player_name: "Alya Tensor".to_string(),
            level: 14,
            level_title: "Arsitek Jaringan".to_string(),
            rooms_defeated: 28,
            total_rooms: 28,
            bosses_defeated: 5,
            total_bosses: 5,
            relics_count: 20,
            total_relics: 20,
            tower_quests_completed: 7,
            total_tower_quests: 7,
            specialization: Some("TinyML".to_string()),
            issued_date_formatted: "31 Oktober 2026".to_string(),
        });
        assert!(cert_text.contains("PIAGAM PENYELESAI PETA BELAJAR"));
        assert!(cert_text.contains("TinyML"));
        assert!(cert_text.contains(&f6.certificate.honest_note));
        assert!(cert_text.contains(&f6.certificate.closing));

        // Dialog monster: ada untuk 28 topik, fallback bila tidak dikenal.
        assert!(get_monster_dialogue("topic-1", "").contains("pecahan"));
        assert_eq!(get_monster_dialogue("topic-99", "Rasa"), "Rasa");

        // Topik terlemah: skor terendah dulu, lalu nomor topik.
        let mut scores: BTreeMap<String, i64> = BTreeMap::new();
        scores.insert("topic-3".to_string(), 60);
        scores.insert("topic-1".to_string(), 60);
        scores.insert("topic-2".to_string(), 100);
        let weakest = get_weakest_topics(&scores, 5);
        assert_eq!(weakest.len(), 3);
        assert_eq!(weakest[0].topic_id, "topic-1");
        assert!(weakest[0].needs_review);
        assert_eq!(weakest[1].topic_id, "topic-3");
        assert_eq!(weakest[2].topic_id, "topic-2");
        assert!(!weakest[2].needs_review);
    }
}
