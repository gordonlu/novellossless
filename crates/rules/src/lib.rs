use anyhow::Result;
use novellossless_storage::{NewContinuityIssue, ProjectChunk, Storage, WorldRule};
use regex::Regex;
use serde_json::json;

pub struct RuleEngine;

impl RuleEngine {
    /// Extract candidate rules from chunk text using pattern matching.
    /// Looks for constraint-like sentences: "不能/无法/不可/禁止/不得/只有...才能/从来/从未"
    pub fn extract_candidates(
        project_id: &str,
        chunks: &[ProjectChunk],
        storage: &Storage,
    ) -> Result<Vec<String>> {
        let mut created_ids = Vec::new();
        let prohibition_re = Regex::new(r"([\p{Han}]{2,20})(?:不能|无法|不可|禁止|不得|不允许|从不|从未)([\p{Han}]{2,40})")?;
        let prerequisite_re = Regex::new(r"只有([\p{Han}]{2,20})(?:才|才能)([\p{Han}]{2,40})")?;
        let now = chrono::Utc::now().to_rfc3339();

        for chunk in chunks {
            for cap in prohibition_re.captures_iter(&chunk.content) {
                let subject = cap.get(1).map(|m| m.as_str()).unwrap_or("");
                let action = cap.get(2).map(|m| m.as_str()).unwrap_or("");
                let name = format!("{subject}不能{action}");
                let rule = WorldRule {
                    id: uuid::Uuid::new_v4().to_string(),
                    project_id: project_id.to_string(),
                    name,
                    description: format!("从正文抽取的约束规则: {subject}不能{action}"),
                    rule_type: "extracted".to_string(),
                    keywords_json: json!([subject, action]).to_string(),
                    positive: true,
                    source_chunk_id: Some(chunk.chunk_id.clone()),
                    confidence: 50,
                    status: "candidate".to_string(),
                    created_at: now.clone(),
                    updated_at: now.clone(),
                };
                storage.upsert_rule(&rule)?;
                created_ids.push(rule.id);
            }

            for cap in prerequisite_re.captures_iter(&chunk.content) {
                let condition = cap.get(1).map(|m| m.as_str()).unwrap_or("");
                let result = cap.get(2).map(|m| m.as_str()).unwrap_or("");
                let name = format!("只有{condition}才能{result}");
                let rule = WorldRule {
                    id: uuid::Uuid::new_v4().to_string(),
                    project_id: project_id.to_string(),
                    name,
                    description: format!("前提约束: 只有{condition}才能{result}"),
                    rule_type: "extracted".to_string(),
                    keywords_json: json!([condition, result]).to_string(),
                    positive: true,
                    source_chunk_id: Some(chunk.chunk_id.clone()),
                    confidence: 50,
                    status: "candidate".to_string(),
                    created_at: now.clone(),
                    updated_at: now.clone(),
                };
                storage.upsert_rule(&rule)?;
                created_ids.push(rule.id);
            }
        }

        Ok(created_ids)
    }

    /// Check chunks against active rules and return issues.
    /// For `positive` rules, find chunks where rule keywords appear together
    /// with contradictory language.
    pub fn check_conflicts(
        chunks: &[ProjectChunk],
        rules: &[WorldRule],
    ) -> Vec<NewContinuityIssue> {
        let mut issues = Vec::new();
        let contradiction_words = ["却", "但是", "然而", "居然", "竟然", "还是", "照常", "依然"];

        for rule in rules {
            if rule.status != "active" {
                continue;
            }
            let keywords: Vec<String> = serde_json::from_str(&rule.keywords_json).unwrap_or_default();
            if keywords.is_empty() {
                continue;
            }

            for chunk in chunks {
                // Check if all keywords appear in this chunk
                let all_keywords_present = keywords.iter().all(|kw| chunk.content.contains(kw.as_str()));
                if !all_keywords_present {
                    continue;
                }

                // Check for contradiction words
                let has_contradiction = contradiction_words.iter()
                    .any(|cw| chunk.content.contains(cw));
                if !has_contradiction {
                    continue;
                }

                let kw_list = keywords.join(", ");
                issues.push(NewContinuityIssue {
                    issue_type: "rule_conflict".to_string(),
                    severity: "high".to_string(),
                    title: format!("可能违反规则「{}」", rule.name),
                    description: format!(
                        "规则「{}」要求关键字「{}」一致，但正文中出现了看似矛盾的表述。",
                        rule.name, kw_list
                    ),
                    evidence_json: serde_json::to_string(&json!({
                        "rule_id": rule.id,
                        "rule_name": rule.name,
                        "chunk_id": chunk.chunk_id,
                        "snippet": chunk.content.chars().take(80).collect::<String>(),
                    })).unwrap_or_default(),
                    suggested_actions_json: serde_json::to_string(&json!([
                        "标记为规则例外",
                        "接受为正式设定变更",
                        "标记为误报"
                    ])).unwrap_or_default(),
                });
            }
        }

        issues
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use novellossless_storage::{NewChunk, NewDocument, Storage, WorldRule};

    fn test_storage_with_project(name: &str) -> Result<(Storage, String)> {
        let storage = Storage::open_memory()?;
        let project = storage.create_project(name, &format!("/tmp/{name}"))?;
        Ok((storage, project.id))
    }

    fn insert_document_chunk(storage: &Storage, project_id: &str, content: &str) -> Result<ProjectChunk> {
        let doc = NewDocument {
            path: "001.txt".into(),
            kind: "novel".into(),
            title: "第一章".into(),
            chapter_count: 1,
            content_hash: "h1".into(),
            word_count: 6,
            encoding: "utf-8".into(),
        };
        storage.upsert_document_with_chunks(
            project_id,
            &doc,
            &[NewChunk {
                chunk_index: 0,
                title: "第一章".into(),
                start_offset: 0,
                end_offset: content.len() as i64,
                content: content.into(),
                content_hash: "h1".into(),
                word_count: 6,
            }],
        )?;
        let chunks = storage.project_chunks(project_id)?;
        Ok(chunks.into_iter().next().unwrap())
    }

    #[test]
    fn extracts_prohibition_rules() -> Result<()> {
        let (storage, pid) = test_storage_with_project("extract_test")?;
        let chunk = insert_document_chunk(&storage, &pid, "魔法不能凭空制造生命。")?;
        let ids = RuleEngine::extract_candidates(&pid, &[chunk], &storage)?;
        assert_eq!(ids.len(), 1);
        let rules = storage.list_rules(&pid)?;
        assert_eq!(rules[0].name, "魔法不能凭空制造生命");
        Ok(())
    }

    #[test]
    fn detects_rule_violation() -> Result<()> {
        let chunks = vec![ProjectChunk {
            document_id: "d1".into(), chunk_id: "c1".into(), document_path: "001.txt".into(),
            chunk_index: 0, title: "第一章".into(),
            content: "魔法却还是凭空制造了一个生命。".into(),
            start_offset: 0, end_offset: 16, word_count: 10, content_hash: "h1".into(),
        }];
        let rules = vec![WorldRule {
            id: "r1".into(), project_id: "p1".into(),
            name: "魔法不能凭空制造生命".into(), description: String::new(),
            rule_type: "world".into(), keywords_json: r#"["魔法","生命","制造"]"#.into(),
            positive: true, source_chunk_id: None, confidence: 100,
            status: "active".into(), created_at: "now".into(), updated_at: "now".into(),
        }];
        let issues = RuleEngine::check_conflicts(&chunks, &rules);
        assert!(!issues.is_empty());
        assert_eq!(issues[0].issue_type, "rule_conflict");
        Ok(())
    }

    #[test]
    fn no_false_positive_on_compliant_text() -> Result<()> {
        let chunks = vec![ProjectChunk {
            document_id: "d1".into(), chunk_id: "c1".into(), document_path: "001.txt".into(),
            chunk_index: 0, title: "第一章".into(),
            content: "他严格遵循规则，从未用魔法制造生命。".into(),
            start_offset: 0, end_offset: 18, word_count: 8, content_hash: "h1".into(),
        }];
        let rules = vec![WorldRule {
            id: "r1".into(), project_id: "p1".into(),
            name: "魔法不能凭空制造生命".into(), description: String::new(),
            rule_type: "world".into(), keywords_json: r#"["魔法","生命","创造"]"#.into(),
            positive: true, source_chunk_id: None, confidence: 100,
            status: "active".into(), created_at: "now".into(), updated_at: "now".into(),
        }];
        let issues = RuleEngine::check_conflicts(&chunks, &rules);
        assert!(issues.is_empty());
        Ok(())
    }
}
