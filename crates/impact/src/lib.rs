use anyhow::Result;
use novellossless_storage::{NewRevisionTask, ProjectChunk, Storage};
use serde_json::json;

pub struct RevisionImpact {
    pub affected_nodes: Vec<String>,
    pub affected_foreshadows: Vec<String>,
    pub affected_rules: Vec<String>,
    pub summary: String,
}

pub struct ImpactAnalyzer;

impl ImpactAnalyzer {
    pub fn analyze(
        project_id: &str,
        old_chunks: &[ProjectChunk],
        new_chunks: &[ProjectChunk],
        storage: &Storage,
    ) -> Result<RevisionImpact> {
        let removed_chunk_ids: Vec<&str> = old_chunks
            .iter()
            .filter(|oc| !new_chunks.iter().any(|nc| nc.chunk_id == oc.chunk_id))
            .map(|c| c.chunk_id.as_str())
            .collect();

        let mut affected_nodes = Vec::new();
        let mut affected_foreshadows = Vec::new();
        let mut affected_rules = Vec::new();

        // Query storage for references to removed chunks
        let nodes = storage.list_narrative_nodes(project_id, None, 1000)?;
        for node in &nodes {
            if removed_chunk_ids.contains(&node.source_chunk_id.as_str()) {
                affected_nodes.push(format!("{} ({}, {})", node.name, node.node_type, node.id));
            }
        }

        let foreshadows = storage.list_foreshadow_items(project_id, 1000)?;
        for f in &foreshadows {
            if removed_chunk_ids.contains(&f.source_chunk_id.as_str()) {
                affected_foreshadows.push(format!("{} ({})", f.title, f.id));
            }
        }

        let rules = storage.list_rules(project_id)?;
        for rule in &rules {
            if let Some(ref scid) = rule.source_chunk_id {
                if removed_chunk_ids.contains(&scid.as_str()) {
                    affected_rules.push(format!("{} ({})", rule.name, rule.id));
                }
            }
        }

        // Create tasks for affected items
        if !affected_nodes.is_empty() || !affected_foreshadows.is_empty() || !affected_rules.is_empty() {
            let summary = format!(
                "修改影响: {} 个人物/地点/物件, {} 个伏笔, {} 条规则",
                affected_nodes.len(), affected_foreshadows.len(), affected_rules.len()
            );
            let _ = storage.create_task(&NewRevisionTask {
                project_id: project_id.to_string(),
                title: summary.clone(),
                task_type: "revision_impact".to_string(),
                priority: "medium".to_string(),
                source_issue_id: None,
                source_foreshadow_id: None,
                related_chunks_json: serde_json::to_string(&removed_chunk_ids).unwrap_or_default(),
                notes: json!({
                    "affected_nodes": affected_nodes,
                    "affected_foreshadows": affected_foreshadows,
                    "affected_rules": affected_rules,
                }).to_string(),
            })?;

            return Ok(RevisionImpact {
                affected_nodes: affected_nodes.clone(),
                affected_foreshadows: affected_foreshadows.clone(),
                affected_rules: affected_rules.clone(),
                summary,
            });
        }

        Ok(RevisionImpact {
            affected_nodes: Vec::new(),
            affected_foreshadows: Vec::new(),
            affected_rules: Vec::new(),
            summary: "无影响".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use novellossless_storage::{NewNarrativeNode, Storage};

    fn test_storage_with_project(name: &str) -> Result<(Storage, String)> {
        let storage = Storage::open_memory()?;
        let project = storage.create_project(name, &format!("/tmp/{name}"))?;
        Ok((storage, project.id))
    }

    #[test]
    fn detects_removed_chunk_referenced_by_node() -> Result<()> {
        let (storage, pid) = test_storage_with_project("impact_test")?;
        // Seed a chunk and a narrative node referencing it
        let doc_id = storage.upsert_document_with_chunks(
            &pid,
            &novellossless_storage::NewDocument {
                path: "001.txt".into(), kind: "text".into(), title: "第一章".into(),
                chapter_count: 1, content_hash: "h".into(), word_count: 5, encoding: "utf-8".into(),
            },
            &[novellossless_storage::NewChunk {
                chunk_index: 0, title: "第一章".into(), start_offset: 0, end_offset: 10,
                content: "林澈在长安。".into(), content_hash: "ch".into(), word_count: 5,
            }],
        )?;
        let chunks = storage.document_chunks(&doc_id)?;
        let chunk_id = chunks[0].chunk_id.clone();

        storage.upsert_narrative_nodes(&pid, &[NewNarrativeNode {
            node_type: "person".into(), name: "林澈".into(),
            aliases_json: "[]".into(), occurrence_count: 1,
            first_chunk_id: chunk_id.clone(),
            latest_chunk_id: chunk_id.clone(),
            confidence: 80,
        }])?;

        let impact = ImpactAnalyzer::analyze(&pid, &chunks, &[], &storage)?;
        assert!(!impact.affected_nodes.is_empty());
        assert!(impact.affected_nodes[0].contains("林澈"));
        Ok(())
    }

    #[test]
    fn no_impact_when_chunk_unchanged() -> Result<()> {
        let (storage, pid) = test_storage_with_project("impact_none")?;
        let doc_id = storage.upsert_document_with_chunks(
            &pid,
            &novellossless_storage::NewDocument {
                path: "001.txt".into(), kind: "text".into(), title: "第一章".into(),
                chapter_count: 1, content_hash: "h".into(), word_count: 5, encoding: "utf-8".into(),
            },
            &[novellossless_storage::NewChunk {
                chunk_index: 0, title: "第一章".into(), start_offset: 0, end_offset: 10,
                content: "林澈在长安。".into(), content_hash: "ch".into(), word_count: 5,
            }],
        )?;
        let old_chunks = storage.document_chunks(&doc_id)?;
        let new_chunks = vec![ProjectChunk {
            document_id: old_chunks[0].document_id.clone(),
            chunk_id: old_chunks[0].chunk_id.clone(),
            chunk_index: 0, title: "第一章".into(),
            content: "林澈在长安。".into(),
            ..old_chunks[0].clone()
        }];
        let impact = ImpactAnalyzer::analyze(&pid, &old_chunks, &new_chunks, &storage)?;
        assert!(impact.affected_nodes.is_empty());
        Ok(())
    }
}
