pub mod checks;
pub mod knowledge;
pub mod loader;
pub mod manifest;
pub mod metrics;
pub mod rule_engine;

pub use checks::IssueEmitter;
pub use knowledge::KnowledgePackLoader;
pub use loader::ProfileLoader;
pub use manifest::*;
pub use metrics::MetricRegistry;
pub use rule_engine::RuleEngine;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn test_profiles_dir() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().to_path_buf();
        fs::create_dir_all(path.join("common_longform")).unwrap();
        fs::write(
            path.join("common_longform").join("profile.toml"),
            r#"id = "common_longform"
name = "通用长篇"
version = "0.1.0"
description = "适用于绝大多数长篇小说的通用模式""#,
        )
        .unwrap();
        fs::create_dir_all(path.join("shuangwen")).unwrap();
        fs::write(
            path.join("shuangwen").join("profile.toml"),
            r#"id = "shuangwen"
name = "爽文模式"
version = "0.1.0"
description = "监控爽点、升级、打脸、战力和读者反馈"

[metrics]
enabled = ["爽点密度", "冲突频次"]

[checks]
enabled = ["战力倒退检查"]"#,
        )
        .unwrap();
        (dir, path)
    }

    #[test]
    fn profile_loader_discovers_all_profiles() {
        let (_tmp, dir) = test_profiles_dir();
        let manifests = ProfileLoader::load_all(&dir).unwrap();
        assert_eq!(manifests.len(), 2);
        let ids: Vec<&str> = manifests.iter().map(|m| m.id.as_str()).collect();
        assert!(ids.contains(&"common_longform"));
        assert!(ids.contains(&"shuangwen"));
    }

    #[test]
    fn profile_loader_skips_dirs_without_profile_toml() {
        let (_tmp, dir) = test_profiles_dir();
        fs::create_dir_all(dir.join("empty_dir")).unwrap();
        let manifests = ProfileLoader::load_all(&dir).unwrap();
        assert_eq!(manifests.len(), 2);
    }

    #[test]
    fn profile_loader_loads_rules() {
        let (_tmp, dir) = test_profiles_dir();
        fs::write(
            dir.join("common_longform").join("rules.toml"),
            r#"[extractors]
people = true
places = true

[people]
min_name_length = 2"#,
        )
        .unwrap();
        let rules = ProfileLoader::load_rules(&dir, "common_longform")
            .unwrap()
            .expect("rules should be present");
        assert!(rules.extractors.people);
        assert_eq!(rules.people.min_name_length, 2);
    }

    #[test]
    fn profile_loader_loads_metrics_toml() {
        let (_tmp, dir) = test_profiles_dir();
        fs::write(
            dir.join("shuangwen").join("metrics.toml"),
            r#"[[metrics]]
id = "爽点密度"
name = "爽点密度"
description = "每千字爽点词出现次数"
weight = 1.0"#,
        )
        .unwrap();
        let metrics_toml = ProfileLoader::load_metrics_toml(&dir, "shuangwen")
            .unwrap()
            .expect("metrics should be present");
        assert_eq!(metrics_toml.metrics.len(), 1);
        assert_eq!(metrics_toml.metrics[0].id, "爽点密度");
    }

    #[test]
    fn rule_engine_merges_multiple_profiles() {
        let (_tmp, dir) = test_profiles_dir();
        fs::write(
            dir.join("shuangwen").join("rules.toml"),
            r#"[extractors]
shuangwen_metrics = true"#,
        )
        .unwrap();
        fs::write(
            dir.join("common_longform").join("rules.toml"),
            r#"[extractors]
people = true
places = true"#,
        )
        .unwrap();

        let manifests = ProfileLoader::load_all(&dir).unwrap();
        let engine = RuleEngine::merge_rules(&manifests, &dir).unwrap();
        assert!(engine.extractors.people);
        assert!(engine.extractors.shuangwen_metrics);
        assert!(!engine.extractors.history_checks);
    }
}
