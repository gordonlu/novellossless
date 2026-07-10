use crate::knowledge::KnowledgePackIndex;
use crate::manifest::CheckDefinition;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct CheckIssue {
    pub issue_type: String,
    pub severity: String,
    pub title: String,
    pub description: String,
    pub evidence_json: String,
    pub suggested_actions_json: String,
}

pub struct IssueEmitter;

impl IssueEmitter {
    pub fn emit(
        check_defs: &[CheckDefinition],
        chunks: &[&str],
        knowledge: &KnowledgePackIndex,
    ) -> Vec<CheckIssue> {
        let mut issues = Vec::new();
        for check in check_defs {
            match check.id.as_str() {
                "战力倒退检查" => {
                    if let Some(issue) = check_power_regression(check, chunks) {
                        issues.push(issue);
                    }
                }
                "身份地位冲突" => {
                    if let Some(issue) = check_identity_status_conflict(check, chunks) {
                        issues.push(issue);
                    }
                }
                "打脸边际递减" => {
                    if let Some(issue) = check_face_slap_diminishing(check, chunks) {
                        issues.push(issue);
                    }
                }
                "连续低爽点章节" => {
                    if let Some(issue) = check_low_shuangwen_streak(check, chunks) {
                        issues.push(issue);
                    }
                }
                "时代穿帮检查" => {
                    let found = check_anachronism(check, chunks, knowledge);
                    issues.extend(found);
                }
                "官职品级冲突" => {
                    // stub — needs per-person tracking across chapters
                }
                "地名时代检查" => {
                    // stub — needs gazetteer knowledge
                }
                _ => {}
            }
        }
        issues
    }

    pub fn extract_checks(manifests: &[crate::manifest::ProfileManifest]) -> Vec<CheckDefinition> {
        let mut checks = Vec::new();
        for m in manifests {
            for check_id in &m.checks.enabled {
                checks.push(CheckDefinition {
                    id: check_id.clone(),
                    name: check_id.clone(),
                    description: String::new(),
                    profile_id: m.id.clone(),
                    severity: "medium".to_string(),
                });
            }
        }
        checks
    }
}

fn check_power_regression(check: &CheckDefinition, chunks: &[&str]) -> Option<CheckIssue> {
    let high_levels = vec!["金丹", "元婴", "化神", "大乘", "渡劫", "大圆满", "巅峰"];
    let low_levels = vec!["炼气", "筑基", "开光", "融合", "后天", "先天"];

    let mut found_high = false;
    let mut found_low_after_high = false;
    let mut evidence_parts = Vec::new();

    for chunk in chunks {
        let has_high = high_levels.iter().any(|l| chunk.contains(*l));
        let has_low = low_levels.iter().any(|l| chunk.contains(*l));

        if has_high && !found_high {
            found_high = true;
        }
        if found_high && has_low {
            if let Some(level) = low_levels.iter().find(|l| chunk.contains(*l)) {
                found_low_after_high = true;
                evidence_parts.push(format!("检测到高级别后出现低级别「{level}」"));
            }
        }
    }

    if found_low_after_high {
        Some(CheckIssue {
            issue_type: check.id.clone(),
            severity: check.severity.clone(),
            title: "战力倒退检查".to_string(),
            description: "主角在达到高境界后又被描述为低境界，可能存在战力倒退或不一致。"
                .to_string(),
            evidence_json: serde_json::to_string(&evidence_parts).unwrap_or_default(),
            suggested_actions_json: r#"["确认是否为误写","确认是否为隐藏实力","标记为误报"]"#
                .to_string(),
        })
    } else {
        None
    }
}

fn check_low_shuangwen_streak(check: &CheckDefinition, chunks: &[&str]) -> Option<CheckIssue> {
    let shuangwen_keywords = vec!["打脸", "震惊", "碾压", "逆袭", "翻盘", "爆"];
    let mut low_streak = 0;
    for chunk in chunks {
        let has_shuangwen = shuangwen_keywords.iter().any(|kw| chunk.contains(*kw));
        if has_shuangwen {
            low_streak = 0;
        } else {
            low_streak += 1;
        }
        if low_streak >= 3 {
            return Some(CheckIssue {
                issue_type: check.id.clone(),
                severity: "low".to_string(),
                title: "连续低爽点章节".to_string(),
                description: "连续多个章节未检测到爽点词汇，可能节奏偏平。".to_string(),
                evidence_json: format!(r#"["{}个连续章节无爽点"]"#, low_streak),
                suggested_actions_json: r#"["检查当前章节节奏","考虑加入冲突或反转"]"#.to_string(),
            });
        }
    }
    None
}

fn check_anachronism(
    check: &CheckDefinition,
    chunks: &[&str],
    knowledge: &KnowledgePackIndex,
) -> Vec<CheckIssue> {
    let modern_words = [
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
        "蓝牙",
        "WiFi",
    ];
    let mut issues = Vec::new();
    let dynasty_terms = knowledge.terms_for_dynasty("唐");

    for (i, chunk) in chunks.iter().enumerate() {
        if !dynasty_terms.is_empty() {
            let has_dynasty_context = dynasty_terms.iter().any(|t| chunk.contains(t));
            if has_dynasty_context {
                for mw in &modern_words {
                    if chunk.contains(mw) {
                        issues.push(CheckIssue {
                            issue_type: check.id.clone(),
                            severity: "medium".to_string(),
                            title: "时代穿帮检查".to_string(),
                            description: format!(
                                "在唐代背景下检测到现代词汇「{mw}」，可能为时代穿帮。"
                            ),
                            evidence_json: format!(r#"["章节{}: …{}…"]"#, i + 1, mw),
                            suggested_actions_json:
                                r#"["确认是否为故意架空","替换为时代合适用语","标记为误报"]"#
                                    .to_string(),
                        });
                    }
                }
            }
        }
    }
    issues
}

fn is_cjk(c: char) -> bool {
    let cp = c as u32;
    (0x4E00..=0x9FFF).contains(&cp) || (0x3400..=0x4DBF).contains(&cp)
}

fn extract_nearby_names(chunk: &str, term: &str, term_start: usize) -> Vec<String> {
    let mut names = Vec::new();

    let before = &chunk[..term_start];
    let before_window: String = before
        .chars()
        .rev()
        .take(10)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    let after_start = term_start + term.len();
    let after = if after_start < chunk.len() {
        &chunk[after_start..]
    } else {
        ""
    };
    let after_window: String = after.chars().take(10).collect();

    let combined = format!("{}{}", before_window, after_window);
    let combined_chars: Vec<char> = combined.chars().collect();
    let mut i = 0;
    while i < combined_chars.len() {
        if is_cjk(combined_chars[i]) {
            let mut j = i + 1;
            while j < combined_chars.len() && is_cjk(combined_chars[j]) {
                j += 1;
            }
            if j - i >= 2 {
                for start in i..j {
                    for len in 2..=3 {
                        if start + len <= j {
                            let name: String = combined_chars[start..start + len].iter().collect();
                            if !names.contains(&name) {
                                names.push(name);
                            }
                        }
                    }
                }
            }
            i = j;
        } else {
            i += 1;
        }
    }

    names
}

fn check_identity_status_conflict(check: &CheckDefinition, chunks: &[&str]) -> Option<CheckIssue> {
    const STATUS_TIERS: &[(&str, &[&str])] = &[
        (
            "high",
            &[
                "皇帝", "太子", "王爷", "公主", "宗主", "上仙", "掌门", "帝王", "至尊", "大帝",
            ],
        ),
        (
            "medium",
            &[
                "少爷", "小姐", "公子", "将军", "长老", "县令", "知府", "家主", "丞相", "尚书",
            ],
        ),
        (
            "low",
            &[
                "平民", "奴仆", "丫鬟", "乞丐", "散修", "杂役", "下人", "婢女", "小厮", "村童",
            ],
        ),
    ];

    let mut char_tiers: HashMap<String, HashSet<String>> = HashMap::new();
    let mut char_evidence: HashMap<String, Vec<(usize, String, String)>> = HashMap::new();

    for (chunk_idx, chunk) in chunks.iter().enumerate() {
        let mut chunk_tiers: Vec<(String, String)> = Vec::new();

        for &(tier_label, terms) in STATUS_TIERS {
            for term in terms {
                let mut search_start = 0;
                while let Some(pos) = chunk[search_start..].find(term) {
                    let abs_pos = search_start + pos;
                    chunk_tiers.push((tier_label.to_string(), term.to_string()));

                    let nearby = extract_nearby_names(chunk, term, abs_pos);
                    for name in nearby {
                        char_tiers
                            .entry(name.clone())
                            .or_default()
                            .insert(tier_label.to_string());
                        char_evidence.entry(name.clone()).or_default().push((
                            chunk_idx,
                            tier_label.to_string(),
                            term.to_string(),
                        ));
                    }

                    search_start = abs_pos + term.len();
                }
            }
        }

        if chunk.contains("主角") && !chunk_tiers.is_empty() {
            for (tier_label, term) in &chunk_tiers {
                let entry = char_evidence.entry("主角".to_string()).or_default();
                if !entry
                    .iter()
                    .any(|(ci, tl, tm)| *ci == chunk_idx && tl == tier_label && tm == term)
                {
                    entry.push((chunk_idx, tier_label.clone(), term.clone()));
                }
                char_tiers
                    .entry("主角".to_string())
                    .or_default()
                    .insert(tier_label.clone());
            }
        }
    }

    let mut evidence_items: Vec<String> = Vec::new();

    for (name, tiers) in &char_tiers {
        if tiers.len() >= 2 {
            let items = char_evidence.get(name).expect("character without evidence");
            let parts: Vec<String> = items
                .iter()
                .map(|(ci, tl, tm)| format!("第{}章: {}({})", ci + 1, tm, tl))
                .collect();
            evidence_items.push(format!("「{}」身份地位冲突: {}", name, parts.join(" → ")));
        }
    }

    if evidence_items.is_empty() {
        None
    } else {
        Some(CheckIssue {
            issue_type: check.id.clone(),
            severity: "medium".to_string(),
            title: "身份地位冲突".to_string(),
            description: "检测到同一角色在不同章节中出现身份地位不一致。".to_string(),
            evidence_json: serde_json::to_string(&evidence_items).unwrap_or_default(),
            suggested_actions_json:
                r#"["确认是否为伪装身份","确认是否为时间线跳跃","统一角色身份设定"]"#.to_string(),
        })
    }
}

fn check_face_slap_diminishing(check: &CheckDefinition, chunks: &[&str]) -> Option<CheckIssue> {
    const EVENT_KEYWORDS: &[&str] = &[
        "打脸",
        "震惊",
        "碾压",
        "逆袭",
        "翻盘",
        "爆",
        "目瞪口呆",
        "骇然",
        "震撼",
        "跪",
        "全场",
    ];
    const SETUP_KEYWORDS: &[&str] = &["挑衅", "羞辱", "侮辱", "嘲讽"];
    const REACTION_KEYWORDS: &[&str] = &["众人", "全场", "目瞪口呆", "跪", "震惊"];

    let mut events: Vec<(usize, bool, bool, usize)> = Vec::new();

    for (i, chunk) in chunks.iter().enumerate() {
        let kw_count = EVENT_KEYWORDS
            .iter()
            .filter(|kw| chunk.contains(**kw))
            .count();
        if kw_count >= 2 {
            let setup = SETUP_KEYWORDS.iter().any(|kw| chunk.contains(*kw));
            let reaction = REACTION_KEYWORDS.iter().any(|kw| chunk.contains(*kw));
            events.push((i, setup, reaction, kw_count));
        }
    }

    if events.is_empty() {
        return None;
    }

    let mut best_start = 0;
    let mut best_len = 1;
    let mut cur_start = 0;

    for idx in 1..events.len() {
        let prev = events[idx - 1];
        let curr = events[idx];
        if prev.1 == curr.1 && prev.2 == curr.2 && prev.3 == curr.3 {
            let cur_len = idx - cur_start + 1;
            if cur_len > best_len {
                best_len = cur_len;
                best_start = cur_start;
            }
        } else {
            cur_start = idx;
        }
    }

    if best_len < 2 {
        return None;
    }

    let severity = if best_len >= 3 { "high" } else { "medium" };

    let evidence: Vec<String> = (best_start..best_start + best_len)
        .map(|i| {
            let (ch_idx, setup, reaction, kw_count) = events[i];
            format!(
                "第{}章: 模式(铺垫={}, 反应={}, 关键词数={})",
                ch_idx + 1,
                if setup { "有" } else { "无" },
                if reaction { "有" } else { "无" },
                kw_count
            )
        })
        .collect();

    Some(CheckIssue {
        issue_type: check.id.clone(),
        severity: severity.to_string(),
        title: "打脸边际递减".to_string(),
        description: "检测到连续章节的打脸事件模式重复，可能导致边际递减效应。".to_string(),
        evidence_json: serde_json::to_string(&evidence).unwrap_or_default(),
        suggested_actions_json: r#"["调整冲突节奏","增加事件变化","重组章节顺序"]"#.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::CheckDefinition;

    #[test]
    fn detects_power_regression() {
        let check = CheckDefinition {
            id: "战力倒退检查".into(),
            name: "战力倒退检查".into(),
            description: String::new(),
            profile_id: "shuangwen".into(),
            severity: "high".into(),
        };
        let chunks = vec!["主角已是金丹期修为。", "主角被打回原形，变成了炼气期。"];
        let issues = IssueEmitter::emit(&[check], &chunks, &KnowledgePackIndex::default());
        assert!(!issues.is_empty());
        assert_eq!(issues[0].issue_type, "战力倒退检查");
    }

    #[test]
    fn no_power_regression_with_consistent_levels() {
        let check = CheckDefinition {
            id: "战力倒退检查".into(),
            name: "".into(),
            description: String::new(),
            profile_id: "shuangwen".into(),
            severity: "medium".into(),
        };
        let chunks = vec!["主角已是元婴期。", "主角突破到化神期。"];
        let issues = IssueEmitter::emit(&[check], &chunks, &KnowledgePackIndex::default());
        assert!(issues.is_empty());
    }

    #[test]
    fn detects_low_shuangwen_streak() {
        let check = CheckDefinition {
            id: "连续低爽点章节".into(),
            name: "".into(),
            description: String::new(),
            profile_id: "shuangwen".into(),
            severity: "low".into(),
        };
        let chunks = vec!["平淡的叙述。", "继续描写风景。", "人物对话。"];
        let issues = IssueEmitter::emit(&[check], &chunks, &KnowledgePackIndex::default());
        assert!(!issues.is_empty());
        assert_eq!(issues[0].issue_type, "连续低爽点章节");
    }

    #[test]
    fn detects_anachronism_with_knowledge_context() {
        let mut knowledge = KnowledgePackIndex::default();
        knowledge.add_dynasty_terms("唐", &["刺史", "县令"]);
        let check = CheckDefinition {
            id: "时代穿帮检查".into(),
            name: "".into(),
            description: String::new(),
            profile_id: "history".into(),
            severity: "medium".into(),
        };
        let chunks = vec!["刺史大人用手机发了一条微信。"];
        let _issues = IssueEmitter::emit(&[check.clone()], &chunks, &KnowledgePackIndex::default());
        // Without knowledge context, anachronism won't fire
        let issues2 = IssueEmitter::emit(&[check], &chunks, &knowledge);
        assert!(!issues2.is_empty());
        assert!(issues2[0].description.contains("手机"));
    }

    #[test]
    fn detects_identity_status_conflict_different_tiers() {
        let check = CheckDefinition {
            id: "身份地位冲突".into(),
            name: "".into(),
            description: String::new(),
            profile_id: "shuangwen".into(),
            severity: "medium".into(),
        };
        let chunks = vec!["主角是当朝皇帝，万人之上。", "主角沦落街头，成了平民。"];
        let issues = IssueEmitter::emit(&[check], &chunks, &KnowledgePackIndex::default());
        assert!(!issues.is_empty());
        assert_eq!(issues[0].issue_type, "身份地位冲突");
        assert_eq!(issues[0].severity, "medium");
    }

    #[test]
    fn no_status_conflict_consistent() {
        let check = CheckDefinition {
            id: "身份地位冲突".into(),
            name: "".into(),
            description: String::new(),
            profile_id: "shuangwen".into(),
            severity: "medium".into(),
        };
        let chunks = vec!["主角是太子，身份尊贵。", "太子殿下登基称帝。"];
        let issues = IssueEmitter::emit(&[check], &chunks, &KnowledgePackIndex::default());
        assert!(issues.is_empty());
    }

    #[test]
    fn no_status_conflict_no_repeated_name() {
        let check = CheckDefinition {
            id: "身份地位冲突".into(),
            name: "".into(),
            description: String::new(),
            profile_id: "shuangwen".into(),
            severity: "medium".into(),
        };
        let chunks = vec!["皇帝李世民上朝。", "平民张三在种地。"];
        let issues = IssueEmitter::emit(&[check], &chunks, &KnowledgePackIndex::default());
        assert!(issues.is_empty());
    }

    #[test]
    fn detects_face_slap_diminishing() {
        let check = CheckDefinition {
            id: "打脸边际递减".into(),
            name: "".into(),
            description: String::new(),
            profile_id: "shuangwen".into(),
            severity: "medium".into(),
        };
        let chunks = vec![
            "一掌打脸反派，众人震惊得目瞪口呆。",
            "继续打脸，众人震惊跪服。",
            "再次打脸，众人震惊骇然。",
        ];
        let issues = IssueEmitter::emit(&[check], &chunks, &KnowledgePackIndex::default());
        assert!(!issues.is_empty());
        assert_eq!(issues[0].issue_type, "打脸边际递减");
        assert_eq!(issues[0].severity, "high");
    }

    #[test]
    fn no_diminishing_when_unique_patterns() {
        let check = CheckDefinition {
            id: "打脸边际递减".into(),
            name: "".into(),
            description: String::new(),
            profile_id: "shuangwen".into(),
            severity: "medium".into(),
        };
        let chunks = vec!["打脸。震惊。", "一巴掌打脸，众人震惊。原来是个挑衅。"];
        let issues = IssueEmitter::emit(&[check], &chunks, &KnowledgePackIndex::default());
        assert!(issues.is_empty());
    }

    #[test]
    fn face_slap_diminishing_medium_vs_high() {
        let check = CheckDefinition {
            id: "打脸边际递减".into(),
            name: "".into(),
            description: String::new(),
            profile_id: "shuangwen".into(),
            severity: "medium".into(),
        };
        let chunks2 = vec!["打脸。震惊。", "打脸。全场。"];
        let issues2 =
            IssueEmitter::emit(&[check.clone()], &chunks2, &KnowledgePackIndex::default());
        assert!(!issues2.is_empty());
        assert_eq!(issues2[0].severity, "medium");

        let chunks3 = vec!["打脸。震惊。", "打脸。全场。", "打脸。目瞪口呆。"];
        let issues3 = IssueEmitter::emit(&[check], &chunks3, &KnowledgePackIndex::default());
        assert!(!issues3.is_empty());
        assert_eq!(issues3[0].severity, "high");
    }
}
