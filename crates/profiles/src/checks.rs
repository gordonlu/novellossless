use crate::knowledge::KnowledgePackIndex;
use crate::manifest::CheckDefinition;

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
                    // stub for now
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
}
