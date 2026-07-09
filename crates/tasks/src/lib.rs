use anyhow::Result;
use novellossless_storage::{ContinuityIssue, ForeshadowItem, NewRevisionTask, Storage};

pub struct TaskManager;

impl TaskManager {
    pub fn auto_create_from_issues(
        project_id: &str,
        issues: &[ContinuityIssue],
        foreshadows: &[ForeshadowItem],
        storage: &Storage,
    ) -> Result<Vec<String>> {
        let mut created_ids = Vec::new();
        let existing = storage.list_tasks(project_id)?;

        for issue in issues {
            if issue.severity != "high" {
                continue;
            }
            let is_duplicate = existing
                .iter()
                .any(|t| t.source_issue_id.as_deref() == Some(&issue.id));
            if !is_duplicate {
                let id = storage.create_task(&NewRevisionTask {
                    project_id: project_id.to_string(),
                    title: format!("[冲突] {}", issue.title),
                    task_type: "conflict".to_string(),
                    priority: issue.severity.clone(),
                    source_issue_id: Some(issue.id.clone()),
                    source_foreshadow_id: None,
                    related_chunks_json: String::new(),
                    notes: String::new(),
                })?;
                created_ids.push(id);
            }
        }

        for f in foreshadows {
            if f.risk_level != "high" {
                continue;
            }
            let is_duplicate = existing
                .iter()
                .any(|t| t.source_foreshadow_id.as_deref() == Some(&f.id));
            if !is_duplicate {
                let id = storage.create_task(&NewRevisionTask {
                    project_id: project_id.to_string(),
                    title: format!("[伏笔] {}", f.title),
                    task_type: "foreshadow".to_string(),
                    priority: "medium".to_string(),
                    source_issue_id: None,
                    source_foreshadow_id: Some(f.id.clone()),
                    related_chunks_json: String::new(),
                    notes: String::new(),
                })?;
                created_ids.push(id);
            }
        }

        Ok(created_ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use novellossless_storage::{ContinuityIssue, Storage};

    fn test_storage_with_project(name: &str) -> Result<(Storage, String)> {
        let storage = Storage::open_memory()?;
        let project = storage.create_project(name, &format!("/tmp/{name}"))?;
        Ok((storage, project.id))
    }

    #[test]
    fn auto_creates_from_high_severity_issue() -> Result<()> {
        let (storage, pid) = test_storage_with_project("auto_task_test")?;
        let issues = vec![ContinuityIssue {
            id: "i1".into(),
            project_id: pid.clone(),
            issue_type: "rule_conflict".into(),
            severity: "high".into(),
            title: "战力倒退".into(),
            description: String::new(),
            evidence_json: String::new(),
            suggested_actions_json: String::new(),
            status: "open".into(),
        }];
        let ids = TaskManager::auto_create_from_issues(&pid, &issues, &[], &storage)?;
        assert_eq!(ids.len(), 1);
        let tasks = storage.list_tasks(&pid)?;
        assert_eq!(tasks[0].task_type, "conflict");
        Ok(())
    }

    #[test]
    fn skips_low_severity_issues() -> Result<()> {
        let (storage, pid) = test_storage_with_project("skip_low")?;
        let issues = vec![ContinuityIssue {
            id: "i2".into(),
            project_id: pid.clone(),
            issue_type: "repeat_expression".into(),
            severity: "low".into(),
            title: "重复".into(),
            description: String::new(),
            evidence_json: String::new(),
            suggested_actions_json: String::new(),
            status: "open".into(),
        }];
        let ids = TaskManager::auto_create_from_issues(&pid, &issues, &[], &storage)?;
        assert!(ids.is_empty());
        Ok(())
    }

    #[test]
    fn deduplicates_on_second_call() -> Result<()> {
        let (storage, pid) = test_storage_with_project("dedup")?;
        let issues = vec![ContinuityIssue {
            id: "i3".into(),
            project_id: pid.clone(),
            issue_type: "rule_conflict".into(),
            severity: "high".into(),
            title: "冲突".into(),
            description: String::new(),
            evidence_json: String::new(),
            suggested_actions_json: String::new(),
            status: "open".into(),
        }];
        TaskManager::auto_create_from_issues(&pid, &issues, &[], &storage)?;
        let ids = TaskManager::auto_create_from_issues(&pid, &issues, &[], &storage)?;
        assert!(ids.is_empty(), "should not create duplicate");
        Ok(())
    }

    #[test]
    fn auto_create_empty_inputs() -> Result<()> {
        let (storage, pid) = test_storage_with_project("empty_auto")?;
        let ids = TaskManager::auto_create_from_issues(&pid, &[], &[], &storage)?;
        assert!(ids.is_empty());
        Ok(())
    }

    #[test]
    fn auto_create_from_foreshadow_creates_task() -> Result<()> {
        let (storage, pid) = test_storage_with_project("foreshadow_task")?;
        let foreshadows = vec![ForeshadowItem {
            id: "f1".into(),
            title: "铜钥匙的秘密".into(),
            foreshadow_type: "mystery".into(),
            status: "candidate".into(),
            risk_level: "high".into(),
            source_chunk_id: "c1".into(),
            source_title: "第一章".into(),
            source_path: "001.txt".into(),
            evidence: "铜钥匙能打开密室。".into(),
        }];
        let ids = TaskManager::auto_create_from_issues(&pid, &[], &foreshadows, &storage)?;
        assert_eq!(ids.len(), 1);
        let tasks = storage.list_tasks(&pid)?;
        assert_eq!(tasks[0].task_type, "foreshadow");
        Ok(())
    }
}
