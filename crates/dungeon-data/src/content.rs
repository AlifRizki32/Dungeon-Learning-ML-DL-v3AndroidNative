//! Model serde untuk konten spellbook & Menara Master v2
//! (`src/data/floor*.content.json`, dipakai ulang apa adanya).
//!
//! File JSON disalin byte-per-byte dari v2 ke `assets/`; struct di sini hanya
//! memodelkan skemanya. `reviewStatus` (`needs-human-review`) ikut terbawa
//! apa adanya, sesuai spec Bagian 3.

use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// Tingkat kesulitan soal kuis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Difficulty {
    Mudah,
    Sedang,
    Sulit,
}

/// Satu mantra (spell) per topik.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Spell {
    pub id: String,
    pub name: String,
    pub formulas: Vec<String>,
}

/// Naskah bacaan (scroll) per topik.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scroll {
    pub paragraphs: Vec<String>,
    #[serde(rename = "workedExample")]
    pub worked_example: String,
    #[serde(rename = "commonMistake")]
    pub common_mistake: String,
}

/// Satu referensi belajar (URL boleh kosong).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContentReference {
    pub label: String,
    pub url: Option<String>,
}

/// Satu soal kuis spellbook.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuizQuestion {
    pub id: String,
    pub difficulty: Difficulty,
    pub question: String,
    pub options: Vec<String>,
    #[serde(rename = "correctIndex")]
    pub correct_index: usize,
    pub explanation: String,
}

/// Konten satu topik di file konten lantai (Lantai 1-5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopicContent {
    #[serde(rename = "topicId")]
    pub topic_id: u32,
    /// Nama relik Guild Quest — hanya ada di Lantai 5.
    #[serde(default)]
    pub relic: Option<String>,
    /// Petunjuk bukti Guild Quest — hanya ada di Lantai 5.
    #[serde(rename = "evidenceHint", default)]
    pub evidence_hint: Option<String>,
    /// Mini-lab numerik — hanya ada di beberapa topik Lantai 3-4.
    #[serde(rename = "miniLab", default)]
    pub mini_lab: Option<String>,
    pub spell: Spell,
    pub scroll: Scroll,
    pub references: Vec<ContentReference>,
    pub quiz: Vec<QuizQuestion>,
}

/// Satu varian soal fase boss (A/B).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BossVariant {
    /// Override spellId varian (mis. "Gema Lantai 1"); bila kosong,
    /// pakai `spell_id` fasenya.
    #[serde(rename = "spellId", default)]
    pub spell_id: Option<String>,
    pub scenario: String,
    pub options: Vec<String>,
    #[serde(rename = "correctIndex")]
    pub correct_index: usize,
    pub explanation: String,
}

/// Satu fase pertarungan boss.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BossPhase {
    pub phase: u32,
    #[serde(rename = "topicId")]
    pub topic_id: u32,
    #[serde(rename = "spellId")]
    pub spell_id: String,
    /// Judul fase (mis. "Gema Lantai 1"); hanya ada di fase gema.
    #[serde(default)]
    pub title: Option<String>,
    pub variants: Vec<BossVariant>,
}

/// Boss satu lantai.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BossContent {
    pub name: String,
    pub phases: Vec<BossPhase>,
}

/// File konten satu lantai (Lantai 1-5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FloorContentFile {
    #[serde(rename = "reviewStatus")]
    pub review_status: String,
    pub floor: u32,
    pub topics: Vec<TopicContent>,
    pub boss: BossContent,
}

/// Satu langkah quest Menara Master.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TowerQuestStep {
    pub id: String,
    pub text: String,
}

/// Satu quest Menara Master (Lantai 6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TowerQuestContent {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub xp: u32,
    pub relic: String,
    #[serde(rename = "evidenceHint")]
    pub evidence_hint: String,
    pub steps: Vec<TowerQuestStep>,
    pub references: Vec<ContentReference>,
}

/// Satu prompt refleksi Penjaga Puncak.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReflectionPrompt {
    pub id: String,
    pub text: String,
}

/// Penjaga Puncak (Cermin Puncak) Lantai 6.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Guardian {
    pub name: String,
    pub intro: String,
    #[serde(rename = "reflectionPrompts")]
    pub reflection_prompts: Vec<ReflectionPrompt>,
}

/// Piagam kelulusan Menara Master.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Certificate {
    pub title: String,
    #[serde(rename = "honestNote")]
    pub honest_note: String,
    pub closing: String,
}

/// File konten Lantai 6 (Menara Master).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Floor6ContentFile {
    #[serde(rename = "reviewStatus")]
    pub review_status: String,
    pub floor: u32,
    #[serde(rename = "stepsAreSuggestions")]
    pub steps_are_suggestions: bool,
    pub intro: String,
    #[serde(rename = "specializationNote")]
    pub specialization_note: String,
    pub specializations: Vec<String>,
    pub quests: Vec<TowerQuestContent>,
    pub guardian: Guardian,
    pub certificate: Certificate,
}

static FLOOR_CONTENTS: [OnceLock<FloorContentFile>; 5] = [
    OnceLock::new(),
    OnceLock::new(),
    OnceLock::new(),
    OnceLock::new(),
    OnceLock::new(),
];

static FLOOR6_CONTENT: OnceLock<Floor6ContentFile> = OnceLock::new();

/// Memuat konten satu lantai (1-5) dari aset bundel; `None` untuk lantai lain.
pub fn floor_content(floor_id: u32) -> Option<&'static FloorContentFile> {
    if !(1..=5).contains(&floor_id) {
        return None;
    }
    let idx = (floor_id - 1) as usize;
    let json: &'static str = match floor_id {
        1 => include_str!("../assets/floor1.content.json"),
        2 => include_str!("../assets/floor2.content.json"),
        3 => include_str!("../assets/floor3.content.json"),
        4 => include_str!("../assets/floor4.content.json"),
        _ => include_str!("../assets/floor5.content.json"),
    };
    Some(FLOOR_CONTENTS[idx].get_or_init(|| {
        serde_json::from_str(json).unwrap_or_else(|e| {
            panic!("assets/floor{floor_id}.content.json rusak: {e}")
        })
    }))
}

/// Memuat konten Lantai 6 (Menara Master) dari aset bundel.
pub fn floor6_content() -> &'static Floor6ContentFile {
    FLOOR6_CONTENT.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/floor6.content.json"))
            .expect("assets/floor6.content.json rusak: tidak bisa di-parse")
    })
}

/// Semua file konten Lantai 1-5, urutan lantai.
pub fn all_floor_contents() -> Vec<&'static FloorContentFile> {
    (1..=5).filter_map(floor_content).collect()
}
