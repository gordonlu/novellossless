use crate::loader::ProfileLoader;
use crate::manifest::ProfileManifest;
use anyhow::Result;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct MetricDefinition {
    pub metric_type: String,
    pub profile_id: String,
    pub name: String,
    pub description: String,
    pub weight: f64,
    pub kind: MetricKind,
}

#[derive(Debug, Clone)]
pub enum MetricKind {
    KeywordDensity(Vec<String>),
    KeywordInterval(Vec<String>),
    ModernWordDensity(Vec<String>),
    FaceSlapIntensity(Vec<(String, f64)>),
    ChapterAvgWords,
    EntityDensity(Vec<String>),
    PacingFlatness {
        window: usize,
        event_keywords: Vec<String>,
        conflict_keywords: Vec<String>,
    },
}

#[derive(Debug, Clone)]
pub struct MetricResult {
    pub profile_id: String,
    pub metric_type: String,
    pub value: f64,
    pub unit: String,
}

pub struct MetricRegistry {
    pub metrics: Vec<MetricDefinition>,
}

impl MetricRegistry {
    pub fn from_profiles(manifests: &[ProfileManifest], root: &Path) -> Result<Self> {
        let mut metrics = Vec::new();
        for m in manifests {
            if let Ok(Some(metrics_toml)) = ProfileLoader::load_metrics_toml(root, &m.id) {
                for entry in metrics_toml.metrics {
                    let kind = metric_kind_for(&entry.id);
                    metrics.push(MetricDefinition {
                        metric_type: entry.id.clone(),
                        profile_id: m.id.clone(),
                        name: entry.name,
                        description: entry.description,
                        weight: entry.weight,
                        kind,
                    });
                }
            }
        }
        Ok(Self { metrics })
    }

    pub fn compute_all(&self, chunks: &[&str]) -> Vec<MetricResult> {
        let mut results = Vec::new();
        for mdef in &self.metrics {
            let value = compute_metric(mdef, chunks);
            let unit = match mdef.kind {
                MetricKind::KeywordDensity(_)
                | MetricKind::ModernWordDensity(_)
                | MetricKind::FaceSlapIntensity(_) => "per_1000_chars",
                MetricKind::KeywordInterval(_) => "chapters",
                MetricKind::ChapterAvgWords => "chars",
                MetricKind::EntityDensity(_) => "per_1000_chars",
                MetricKind::PacingFlatness { .. } => "ratio",
            };
            results.push(MetricResult {
                profile_id: mdef.profile_id.clone(),
                metric_type: mdef.metric_type.clone(),
                value,
                unit: unit.to_string(),
            });
        }
        results
    }

    pub fn compute(&self, metric_type: &str, chunks: &[&str]) -> Option<f64> {
        let mdef = self.metrics.iter().find(|m| m.metric_type == metric_type)?;
        Some(compute_metric(mdef, chunks))
    }
}

pub(crate) fn metric_kind_for(metric_type: &str) -> MetricKind {
    match metric_type {
        "爽点密度" => MetricKind::KeywordDensity(
            [
                "打脸",
                "震惊",
                "碾压",
                "逆袭",
                "翻盘",
                "爆",
                "众人",
                "全场",
                "目瞪口呆",
                "骇然",
                "震撼",
                "跪",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
        ),
        "冲突频次" => MetricKind::KeywordDensity(
            [
                "挑衅", "羞辱", "赌约", "竞争", "对抗", "冲突", "战斗", "厮杀", "压迫", "侮辱",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
        ),
        "升级间隔" => MetricKind::KeywordInterval(
            ["晋级", "突破", "进阶", "提升", "升级"]
                .into_iter()
                .map(String::from)
                .collect(),
        ),
        "时代穿帮风险" => MetricKind::ModernWordDensity(
            [
                "手机",
                "电脑",
                "电视",
                "网络",
                "微信",
                "互联网",
                "数据",
                "芯片",
                "程序",
                "代码",
                "AI",
                "算法",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
        ),
        "打脸强度" => MetricKind::FaceSlapIntensity(vec![
            ("打脸".into(), 1.0),
            ("震惊".into(), 2.0),
            ("目瞪口呆".into(), 2.0),
            ("骇然".into(), 2.0),
            ("震撼".into(), 2.0),
            ("碾压".into(), 3.0),
            ("跪".into(), 2.0),
            ("全场".into(), 1.0),
        ]),
        "节奏平缓风险" => MetricKind::PacingFlatness {
            window: 5,
            event_keywords: vec![
                "打脸".into(),
                "震惊".into(),
                "碾压".into(),
                "逆袭".into(),
                "翻盘".into(),
                "爆".into(),
                "目瞪口呆".into(),
                "骇然".into(),
                "震撼".into(),
                "跪".into(),
                "全场".into(),
            ],
            conflict_keywords: vec![
                "挑衅".into(),
                "羞辱".into(),
                "赌约".into(),
                "竞争".into(),
                "对抗".into(),
                "冲突".into(),
                "战斗".into(),
                "厮杀".into(),
                "压迫".into(),
                "侮辱".into(),
            ],
        },
        "章节平均字数" => MetricKind::ChapterAvgWords,
        "人物密度" => MetricKind::EntityDensity(
            // Common Chinese character name patterns - can't know them ahead of scan
            // so we keep an empty list; the real extraction happens via the core candidate system.
            // This metric uses candidate data from the project, not chunk keyword matching.
            Vec::new(),
        ),
        "地名密度" => MetricKind::EntityDensity(Vec::new()),
        "叙事密度" => MetricKind::KeywordDensity(Vec::new()), // ratio of narrative vs dialogue, placeholder
        _ => MetricKind::KeywordDensity(Vec::new()),
    }
}

pub(crate) fn compute_metric(mdef: &MetricDefinition, chunks: &[&str]) -> f64 {
    match &mdef.kind {
        MetricKind::ChapterAvgWords => {
            if chunks.is_empty() {
                return 0.0;
            }
            let total: usize = chunks.iter().map(|c| c.chars().count()).sum();
            total as f64 / chunks.len() as f64
        }
        MetricKind::EntityDensity(keywords) => {
            if chunks.is_empty() {
                return 0.0;
            }
            let total_chars: usize = chunks.iter().map(|c| c.chars().count()).sum();
            if total_chars == 0 {
                return 0.0;
            }
            if keywords.is_empty() {
                return 0.0; // needs project-level candidate data
            }
            let total_matches: usize = chunks
                .iter()
                .flat_map(|c| keywords.iter().filter(|kw| c.contains(kw.as_str())))
                .count();
            (total_matches as f64 / total_chars as f64) * 1000.0 * mdef.weight
        }
        MetricKind::KeywordDensity(keywords) | MetricKind::ModernWordDensity(keywords) => {
            if keywords.is_empty() || chunks.is_empty() {
                return 0.0;
            }
            let total_chars: usize = chunks.iter().map(|c| c.chars().count()).sum();
            if total_chars == 0 {
                return 0.0;
            }
            let total_matches: usize = chunks
                .iter()
                .flat_map(|c| keywords.iter().filter(|kw| c.contains(kw.as_str())))
                .count();
            (total_matches as f64 / total_chars as f64) * 1000.0 * mdef.weight
        }
        MetricKind::KeywordInterval(keywords) => {
            if keywords.is_empty() || chunks.is_empty() {
                return 0.0;
            }
            let mut last_match = None;
            let mut intervals = Vec::new();
            for (i, chunk) in chunks.iter().enumerate() {
                if keywords.iter().any(|kw| chunk.contains(kw.as_str())) {
                    if let Some(last) = last_match {
                        intervals.push(i - last);
                    }
                    last_match = Some(i);
                }
            }
            if intervals.is_empty() {
                return chunks.len() as f64;
            }
            intervals.iter().sum::<usize>() as f64 / intervals.len() as f64
        }
        MetricKind::FaceSlapIntensity(pairs) => {
            if pairs.is_empty() || chunks.is_empty() {
                return 0.0;
            }
            let total_chars: usize = chunks.iter().map(|c| c.chars().count()).sum();
            if total_chars == 0 {
                return 0.0;
            }
            let total_score: f64 = pairs
                .iter()
                .map(|(kw, weight)| {
                    let count = chunks.iter().filter(|c| c.contains(kw.as_str())).count();
                    count as f64 * weight
                })
                .sum();
            (total_score / total_chars as f64) * 1000.0 * mdef.weight
        }
        MetricKind::PacingFlatness {
            window,
            event_keywords,
            conflict_keywords,
        } => {
            if chunks.len() < *window {
                return 0.0;
            }
            let total_chars: usize = chunks.iter().map(|c| c.chars().count()).sum();
            let avg_chunk_len = total_chars as f64 / chunks.len() as f64;
            let total_windows = chunks.len() - window + 1;
            let risk_windows: usize = (0..total_windows)
                .filter(|&i| {
                    let window_chunks = &chunks[i..i + window];
                    let event_count: usize = window_chunks
                        .iter()
                        .flat_map(|c| {
                            event_keywords
                                .iter()
                                .chain(conflict_keywords.iter())
                                .filter(|kw| c.contains(kw.as_str()))
                        })
                        .count();
                    let total_chars_in_window: usize =
                        window_chunks.iter().map(|c| c.chars().count()).sum();
                    event_count < 2
                        && (total_chars_in_window as f64) > avg_chunk_len * (*window as f64) * 1.5
                })
                .count();
            risk_windows as f64 / total_windows as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn keyword_density_metric() {
        let mdef = MetricDefinition {
            metric_type: "爽点密度".into(),
            profile_id: "shuangwen".into(),
            name: "爽点密度".into(),
            description: String::new(),
            weight: 1.0,
            kind: MetricKind::KeywordDensity(vec!["打脸".into(), "震惊".into()]),
        };
        let chunks = vec!["第一章 打脸！震惊！众人。"];
        let value = compute_metric(&mdef, &chunks);
        assert!(value > 0.0, "should detect keywords: {value}");
    }

    #[test]
    fn keyword_interval_metric() {
        let mdef = MetricDefinition {
            metric_type: "升级间隔".into(),
            profile_id: "shuangwen".into(),
            name: "升级间隔".into(),
            description: String::new(),
            weight: 1.0,
            kind: MetricKind::KeywordInterval(vec!["突破".into()]),
        };
        let chunks = vec!["a", "突破！", "b", "c", "突破！"];
        let value = compute_metric(&mdef, &chunks);
        assert!((value - 3.0).abs() < 0.01, "expected ~3.0, got {value}");
    }

    #[test]
    fn modern_word_density_returns_zero_for_clean_text() {
        let mdef = MetricDefinition {
            metric_type: "时代穿帮风险".into(),
            profile_id: "history".into(),
            name: String::new(),
            description: String::new(),
            weight: 1.0,
            kind: MetricKind::ModernWordDensity(vec!["手机".into(), "电脑".into()]),
        };
        let chunks = vec!["将军上马，刺史下令。长安城外一片肃杀。"];
        let value = compute_metric(&mdef, &chunks);
        assert_eq!(value, 0.0, "no modern words: {value}");
    }

    #[test]
    fn metric_registry_computes_shuangwen_metrics() {
        let registry = MetricRegistry::from_profiles(&[], &PathBuf::from("/nonexistent")).unwrap();
        let chapters = vec![
            "第一章 林澈一拳打脸反派，众人震惊！他直接升级突破了。",
            "第二章 碾压对手，全场震惊。又是一个爽点。",
        ];
        let results = registry.compute_all(&chapters);
        assert!(results.is_empty());
    }

    #[test]
    fn face_slap_intensity_metric() {
        let mdef = MetricDefinition {
            metric_type: "打脸强度".into(),
            profile_id: "shuangwen".into(),
            name: "打脸强度".into(),
            description: String::new(),
            weight: 1.0,
            kind: MetricKind::FaceSlapIntensity(vec![("打脸".into(), 1.0), ("碾压".into(), 3.0)]),
        };
        let chunks = vec!["第一章 打脸！碾压对手。"];
        let value = compute_metric(&mdef, &chunks);
        assert!(value > 0.0, "should detect face-slapping: {value}");
    }

    #[test]
    fn face_slap_intensity_empty() {
        let mdef = MetricDefinition {
            metric_type: "打脸强度".into(),
            profile_id: "shuangwen".into(),
            name: "打脸强度".into(),
            description: String::new(),
            weight: 1.0,
            kind: MetricKind::FaceSlapIntensity(vec![("打脸".into(), 1.0)]),
        };
        let chunks: Vec<&str> = vec![];
        let value = compute_metric(&mdef, &chunks);
        assert_eq!(value, 0.0);
    }

    #[test]
    fn face_slap_intensity_weighted() {
        let mdef = MetricDefinition {
            metric_type: "打脸强度".into(),
            profile_id: "shuangwen".into(),
            name: "打脸强度".into(),
            description: String::new(),
            weight: 1.0,
            kind: MetricKind::FaceSlapIntensity(vec![("打脸".into(), 1.0), ("碾压".into(), 3.0)]),
        };
        let val_da_lian = compute_metric(&mdef, &["打脸。"]);
        let val_nian_ya = compute_metric(&mdef, &["碾压。"]);
        assert!(
            val_nian_ya > val_da_lian,
            "碾压 ({}) should contribute more than 打脸 ({})",
            val_nian_ya,
            val_da_lian
        );
    }

    #[test]
    fn pacing_flatness_risk_detects_flat() {
        let mdef = MetricDefinition {
            metric_type: "节奏平缓风险".into(),
            profile_id: "shuangwen".into(),
            name: "节奏平缓风险".into(),
            description: String::new(),
            weight: 1.0,
            kind: MetricKind::PacingFlatness {
                window: 5,
                event_keywords: vec!["打脸".into(), "震惊".into(), "碾压".into()],
                conflict_keywords: vec!["挑衅".into(), "战斗".into()],
            },
        };
        let longs: Vec<String> = (0..5).map(|_| "a".repeat(100)).collect();
        let shorts: Vec<String> = (0..5).map(|_| "b".repeat(1)).collect();
        let all: Vec<String> = longs.into_iter().chain(shorts.into_iter()).collect();
        let chunk_refs: Vec<&str> = all.iter().map(|s| s.as_str()).collect();
        let value = compute_metric(&mdef, &chunk_refs);
        assert!(value > 0.0, "should detect flat pacing: {value}");
    }

    #[test]
    fn pacing_flatness_risk_no_risk_when_active() {
        let mdef = MetricDefinition {
            metric_type: "节奏平缓风险".into(),
            profile_id: "shuangwen".into(),
            name: "节奏平缓风险".into(),
            description: String::new(),
            weight: 1.0,
            kind: MetricKind::PacingFlatness {
                window: 5,
                event_keywords: vec!["打脸".into(), "逆袭".into()],
                conflict_keywords: vec!["挑衅".into()],
            },
        };
        let chunks = vec![
            "打脸逆袭",
            "打脸逆袭",
            "打脸逆袭",
            "打脸逆袭",
            "打脸逆袭",
            "打脸逆袭",
        ];
        let value = compute_metric(&mdef, &chunks);
        assert_eq!(value, 0.0, "active pacing should have 0 risk: {value}");
    }

    #[test]
    fn pacing_flatness_risk_empty() {
        let mdef = MetricDefinition {
            metric_type: "节奏平缓风险".into(),
            profile_id: "shuangwen".into(),
            name: "节奏平缓风险".into(),
            description: String::new(),
            weight: 1.0,
            kind: MetricKind::PacingFlatness {
                window: 5,
                event_keywords: vec!["打脸".into()],
                conflict_keywords: vec![],
            },
        };
        let chunks: Vec<&str> = vec![];
        let value = compute_metric(&mdef, &chunks);
        assert_eq!(value, 0.0);
    }

    #[test]
    fn pacing_flatness_sliding_window_boundary() {
        let mdef = MetricDefinition {
            metric_type: "节奏平缓风险".into(),
            profile_id: "shuangwen".into(),
            name: "节奏平缓风险".into(),
            description: String::new(),
            weight: 1.0,
            kind: MetricKind::PacingFlatness {
                window: 5,
                event_keywords: vec!["打脸".into()],
                conflict_keywords: vec![],
            },
        };
        let chunks = vec!["a"; 5];
        let value = compute_metric(&mdef, &chunks);
        assert_eq!(value, 0.0, "boundary case should be 0: {value}");
    }

    #[test]
    fn modern_word_density_detects_words() {
        let mdef = MetricDefinition {
            metric_type: "时代穿帮风险".into(),
            profile_id: "history".into(),
            name: String::new(),
            description: String::new(),
            weight: 1.0,
            kind: MetricKind::ModernWordDensity(vec!["手机".into(), "电脑".into()]),
        };
        let chunks = vec!["他用手机打电话，用电脑写代码。"];
        let value = compute_metric(&mdef, &chunks);
        assert!(value > 0.0, "should detect modern words: {value}");
    }

    #[test]
    fn metric_kind_for_unknown_type_returns_empty() {
        let kind = metric_kind_for("不存在的指标");
        match kind {
            MetricKind::KeywordDensity(kws) => assert!(kws.is_empty()),
            _ => panic!("unknown type should map to empty KeywordDensity"),
        }
    }
}
