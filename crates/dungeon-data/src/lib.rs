//! `dungeon-data` — model data dan pemuat konten v3 (Fase 2).
//!
//! Semua konten v2 dipakai ulang apa adanya:
//!
//! - `assets/roadmap.json` — konversi mekanis satu kali dari
//!   `src/data/roadmap.js` v2 (Node: import + `JSON.stringify`, tanpa
//!   perubahan isi). Turunan (`ALL_TOPICS`, relik, badge) dihitung di Rust.
//! - `assets/floor*.content.json` — disalin byte-per-byte dari v2.
//! - `assets/lessons/floor*.lessons.json` — disalin byte-per-byte dari v2.
//!
//! Status kurikulum `needs-human-review` ikut apa adanya (spec Bagian 3).
//! Aset dimuat lewat `include_str!` + serde, sekali per proses (`OnceLock`).

pub mod content;
pub mod lessons;
pub mod roadmap;

pub use content::{
    BossContent, BossPhase, BossVariant, Certificate, ContentReference, Difficulty,
    Floor6ContentFile, FloorContentFile, Guardian, QuizQuestion, ReflectionPrompt, Spell,
    Scroll, TowerQuestContent, TowerQuestStep, TopicContent,
};
pub use lessons::{
    BarsItem, DotGridGroup, FloorLessonsFile, LessonChunk, LessonFormula, LessonGuess,
    LessonPracticeActivity, LessonReviewItem, LessonSceneLine, LessonStep, LessonTerm,
    LessonTopicEntry, LessonVisual, LineDef, MatchPair, MatrixDef,
};
pub use roadmap::{
    Avatar, BadgeItem, BossItem, FloorItem, RelicItem, Roadmap, RoadmapAssumptions,
    TopicItem, TowerFloor, TowerQuest, WeeksRange,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roadmap_parses_and_counts_match_v2() {
        let r = roadmap::roadmap();
        assert_eq!(r.floors.len(), 5);
        assert_eq!(roadmap::all_topics().len(), 28);
        // Pembagian topik per lantai: 6 / 5 / 6 / 3 / 8 (spec Bagian 3).
        let counts: Vec<usize> = r.floors.iter().map(|f| f.topics.len()).collect();
        assert_eq!(counts, vec![6, 5, 6, 3, 8]);
        assert_eq!(r.tower_floor.quests.len(), 7);
        assert_eq!(r.avatars.len(), 4);
        // Total minggu mentah 79-104 (formulas.test.ts).
        let raw_min: u32 = r.floors.iter().map(|f| f.weeks.min).sum();
        let raw_max: u32 = r.floors.iter().map(|f| f.weeks.max).sum();
        assert_eq!((raw_min, raw_max), (79, 104));
        assert_eq!(roadmap::all_relics().len(), 13); // 5 boss + 8 guild quest
        assert_eq!(roadmap::all_badges().len(), 5);
        assert_eq!(roadmap::floor_for_topic_number(1), 1);
        assert_eq!(roadmap::floor_for_topic_number(11), 2);
        assert_eq!(roadmap::floor_for_topic_number(17), 3);
        assert_eq!(roadmap::floor_for_topic_number(20), 4);
        assert_eq!(roadmap::floor_for_topic_number(28), 5);
        assert_eq!(roadmap::topic_by_number(1).unwrap().name, "Matematika dasar");
        assert_eq!(roadmap::topic_by_id("topic-28").unwrap().floor_id, 5);
    }

    #[test]
    fn floor_contents_parse_and_match_v2_structure() {
        // Lantai 1: 6 topik, 5 soal per topik, boss Raja Matematika 6 fase x 2 varian.
        let f1 = content::floor_content(1).unwrap();
        assert_eq!(f1.review_status, "needs-human-review");
        assert_eq!(f1.floor, 1);
        assert_eq!(f1.topics.len(), 6);
        assert!(f1.topics.iter().all(|t| t.quiz.len() == 5));
        assert_eq!(f1.boss.name, "Raja Matematika");
        assert_eq!(f1.boss.phases.len(), 6);
        assert!(f1.boss.phases.iter().all(|p| p.variants.len() == 2));

        let f2 = content::floor_content(2).unwrap();
        assert_eq!(f2.boss.name, "Golem Pohon Purba");
        assert_eq!(f2.boss.phases.len(), 6);
        // Fase 6 "Gema Lantai 1": override spellId varian s5 / s2.
        let p6 = &f2.boss.phases[5];
        assert_eq!(p6.title.as_deref(), Some("Gema Lantai 1"));
        assert_eq!(p6.variants[0].spell_id.as_deref(), Some("s5"));
        assert_eq!(p6.variants[1].spell_id.as_deref(), Some("s2"));

        let f3 = content::floor_content(3).unwrap();
        assert_eq!(f3.boss.name, "Hydra Berkepala Banyak");
        assert_eq!(f3.boss.phases.len(), 7);
        assert_eq!(f3.topics[0].mini_lab.as_deref(), Some("neuron"));
        assert_eq!(f3.topics[4].mini_lab.as_deref(), Some("softmax"));

        let f4 = content::floor_content(4).unwrap();
        assert_eq!(f4.boss.name, "Mecha-Core");
        assert_eq!(f4.boss.phases.len(), 5);
        assert_eq!(f4.topics[1].mini_lab.as_deref(), Some("quant"));
        assert_eq!(f4.topics[2].mini_lab.as_deref(), Some("latency"));

        let f5 = content::floor_content(5).unwrap();
        assert_eq!(f5.boss.name, "Raja Deadline");
        assert_eq!(f5.boss.phases.len(), 8);
        assert_eq!(f5.topics.len(), 8);
        assert!(f5.topics.iter().all(|t| t.relic.is_some()));
        assert!(f5
            .topics
            .iter()
            .all(|t| t.evidence_hint.as_ref().is_some_and(|h| !h.is_empty())));

        let f6 = content::floor6_content();
        assert_eq!(f6.review_status, "needs-human-review");
        assert_eq!(f6.floor, 6);
        assert!(f6.steps_are_suggestions);
        assert_eq!(f6.quests.len(), 7);
        assert_eq!(f6.specializations.len(), 5);
        assert_eq!(f6.guardian.name, "Cermin Puncak");
        assert_eq!(f6.guardian.reflection_prompts.len(), 5);
        assert_eq!(f6.certificate.title, "Piagam Penyelesai Peta Belajar");
        assert!(f6.quests.iter().all(|q| q.xp == 150));
        assert!(f6.quests.iter().all(|q| q.steps.len() >= 4));

        assert!(content::floor_content(0).is_none());
        assert!(content::floor_content(6).is_none());
    }

    #[test]
    fn lesson_files_parse_and_counts_match_v2() {
        // Jumlah chunk per lantai (lessonEngine.test.ts): 22/22/23/13/29/19.
        let expected_chunks = [22usize, 22, 23, 13, 29, 19];
        let mut total_review = 0usize;
        for (i, expected) in expected_chunks.iter().enumerate() {
            let floor = (i + 1) as u32;
            let file = lessons::floor_lessons(floor).unwrap();
            assert_eq!(file.review_status, "needs-human-review");
            assert_eq!(file.floor, floor);
            let n: usize = file.topics.iter().map(|t| t.chunks.len()).sum::<usize>()
                + file.quests.iter().map(|q| q.chunks.len()).sum::<usize>();
            assert_eq!(n, *expected, "lantai {floor}");
            total_review += file
                .topics
                .iter()
                .flat_map(|t| t.chunks.iter())
                .map(|c| c.review_items.len())
                .sum::<usize>()
                + file
                    .quests
                    .iter()
                    .flat_map(|q| q.chunks.iter())
                    .map(|c| c.review_items.len())
                    .sum::<usize>();
        }
        // 156 item review di semua lantai (lessonEngine.test.ts).
        assert_eq!(total_review, 156);
        assert!(lessons::floor_lessons(0).is_none());
        assert!(lessons::floor_lessons(7).is_none());
    }
}
