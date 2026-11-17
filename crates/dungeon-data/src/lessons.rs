//! Model serde untuk pelajaran bertahap v2
//! (`src/data/lessons/floor*.lessons.json`, dipakai ulang apa adanya).
//!
//! File JSON disalin byte-per-byte dari v2 ke `assets/lessons/`; struct di sini
//! hanya memodelkan skemanya. `reviewStatus` (`needs-human-review`) ikut
//! terbawa apa adanya, sesuai spec Bagian 3.

use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// Satu istilah glosarium.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LessonTerm {
    pub term: String,
    pub plain: String,
}

/// Satu baris dialog adegan pelajaran.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LessonSceneLine {
    pub who: String,
    pub text: String,
}

/// Tebakan pembuka chunk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LessonGuess {
    pub question: String,
    pub options: Vec<String>,
    #[serde(rename = "correctIndex")]
    pub correct_index: usize,
    pub reveal: String,
}

/// Satu kelompok titik pada visual dotGrid.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DotGridGroup {
    pub id: String,
    pub count: u32,
    pub label: String,
    #[serde(rename = "subsetOf", default)]
    pub subset_of: Option<String>,
}

/// Satu batang pada visual bars.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BarsItem {
    pub id: String,
    pub label: String,
    pub value: f64,
}

/// Garis pada visual linePlot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LineDef {
    pub m: f64,
    pub c: f64,
}

/// Satu matriks pada visual matrixGrid.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatrixDef {
    pub id: String,
    pub rows: Vec<Vec<f64>>,
    #[serde(rename = "rowLabels", default)]
    pub row_labels: Option<Vec<String>>,
    #[serde(rename = "colLabels", default)]
    pub col_labels: Option<Vec<String>>,
}

/// Visual pelajaran — 5 jenis yang ada di konten v2. `Unknown` menjaga
/// parse tetap berhasil bila konten nanti menambah jenis baru.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum LessonVisual {
    #[serde(rename = "dotGrid")]
    DotGrid {
        total: u32,
        columns: u32,
        groups: Vec<DotGridGroup>,
    },
    #[serde(rename = "bars")]
    Bars { items: Vec<BarsItem> },
    #[serde(rename = "linePlot")]
    LinePlot {
        points: Vec<(f64, f64)>,
        #[serde(default)]
        line: Option<LineDef>,
    },
    #[serde(rename = "matrixGrid")]
    MatrixGrid { matrices: Vec<MatrixDef> },
    #[serde(rename = "codeBlock")]
    CodeBlock { lines: Vec<String> },
    #[serde(other)]
    Unknown,
}

/// Satu langkah penjelasan chunk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LessonStep {
    pub text: String,
    pub highlight: Vec<String>,
}

/// Satu pasangan kiri-kanan pada latihan `match`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatchPair {
    pub left: String,
    pub right: String,
}

/// Satu aktivitas latihan — 4 jenis yang ada di konten v2. `Unknown` menjaga
/// parse tetap berhasil bila konten nanti menambah jenis baru.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum LessonPracticeActivity {
    #[serde(rename = "choice")]
    Choice {
        id: String,
        question: String,
        options: Vec<String>,
        #[serde(rename = "correctIndex")]
        correct_index: usize,
        why: String,
    },
    #[serde(rename = "fillNumber")]
    FillNumber {
        id: String,
        question: String,
        answer: f64,
        tolerance: f64,
        unit: String,
        why: String,
    },
    #[serde(rename = "order")]
    Order {
        id: String,
        question: String,
        items: Vec<String>,
        why: String,
    },
    #[serde(rename = "match")]
    Match {
        id: String,
        question: String,
        pairs: Vec<MatchPair>,
        why: String,
    },
    #[serde(other)]
    Unknown,
}

impl LessonPracticeActivity {
    /// Id aktivitas (dipakai pelacak progres chunk).
    pub fn id(&self) -> &str {
        match self {
            LessonPracticeActivity::Choice { id, .. }
            | LessonPracticeActivity::FillNumber { id, .. }
            | LessonPracticeActivity::Order { id, .. }
            | LessonPracticeActivity::Match { id, .. } => id,
            LessonPracticeActivity::Unknown => "",
        }
    }
}

/// Rumus chunk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LessonFormula {
    pub lines: Vec<String>,
    #[serde(rename = "readAloud")]
    pub read_aloud: String,
}

/// Satu kartu review (flashcard) chunk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LessonReviewItem {
    pub id: String,
    pub front: String,
    pub back: String,
}

/// Satu chunk pelajaran.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LessonChunk {
    #[serde(rename = "chunkId")]
    pub chunk_id: String,
    pub title: String,
    pub goal: String,
    /// Mini-lab numerik (`neuron`/`softmax`/`quant`/`latency`).
    #[serde(rename = "miniLab", default)]
    pub mini_lab: Option<String>,
    pub terms: Vec<LessonTerm>,
    pub scene: Vec<LessonSceneLine>,
    pub guess: LessonGuess,
    #[serde(default)]
    pub visual: Option<LessonVisual>,
    pub steps: Vec<LessonStep>,
    pub practice: Vec<LessonPracticeActivity>,
    #[serde(rename = "formulaTitle", default)]
    pub formula_title: Option<String>,
    pub formula: LessonFormula,
    #[serde(rename = "commonMistake")]
    pub common_mistake: String,
    /// Tugas nyata — hanya ada di chunk Lantai 5 dan Lantai 6.
    #[serde(rename = "realTask", default)]
    pub real_task: Option<String>,
    #[serde(rename = "summaryPrompt")]
    pub summary_prompt: String,
    #[serde(rename = "reviewItems")]
    pub review_items: Vec<LessonReviewItem>,
}

/// Entri topik di file pelajaran (Lantai 1-5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LessonTopicEntry {
    #[serde(rename = "topicId")]
    pub topic_id: u32,
    pub chunks: Vec<LessonChunk>,
}

/// Entri quest di file pelajaran (Lantai 6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LessonQuestEntry {
    #[serde(rename = "questId")]
    pub quest_id: String,
    pub chunks: Vec<LessonChunk>,
}

/// Pemandu pelajaran.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LessonGuide {
    pub name: String,
    pub role: String,
}

/// Satu file pelajaran per lantai.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FloorLessonsFile {
    #[serde(rename = "reviewStatus")]
    pub review_status: String,
    pub floor: u32,
    #[serde(rename = "lessonVersion")]
    pub lesson_version: u32,
    pub guide: LessonGuide,
    #[serde(default)]
    pub topics: Vec<LessonTopicEntry>,
    #[serde(default)]
    pub quests: Vec<LessonQuestEntry>,
}

static LESSON_FILES: [OnceLock<FloorLessonsFile>; 6] = [
    OnceLock::new(),
    OnceLock::new(),
    OnceLock::new(),
    OnceLock::new(),
    OnceLock::new(),
    OnceLock::new(),
];

/// Memuat file pelajaran satu lantai (1-6) dari aset bundel; `None` di luar itu.
pub fn floor_lessons(floor: u32) -> Option<&'static FloorLessonsFile> {
    if !(1..=6).contains(&floor) {
        return None;
    }
    let idx = (floor - 1) as usize;
    let json: &'static str = match floor {
        1 => include_str!("../assets/lessons/floor1.lessons.json"),
        2 => include_str!("../assets/lessons/floor2.lessons.json"),
        3 => include_str!("../assets/lessons/floor3.lessons.json"),
        4 => include_str!("../assets/lessons/floor4.lessons.json"),
        5 => include_str!("../assets/lessons/floor5.lessons.json"),
        _ => include_str!("../assets/lessons/floor6.lessons.json"),
    };
    Some(LESSON_FILES[idx].get_or_init(|| {
        serde_json::from_str(json)
            .unwrap_or_else(|e| panic!("assets/lessons/floor{floor}.lessons.json rusak: {e}"))
    }))
}

/// Semua 6 file pelajaran, urutan lantai.
pub fn all_lesson_files() -> Vec<&'static FloorLessonsFile> {
    (1..=6).filter_map(floor_lessons).collect()
}
