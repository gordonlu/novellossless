use crate::loader::ProfileLoader;
use crate::manifest::*;
use anyhow::Result;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct RuleEngine {
    pub extractors: ExtractorRules,
}

impl RuleEngine {
    pub fn merge_rules(manifests: &[ProfileManifest], root: &Path) -> Result<Self> {
        let mut merged = ExtractorRules::default();
        // Start with all false
        merged.people = false;
        merged.places = false;
        merged.items = false;
        merged.foreshadows = false;
        merged.eye_color_conflicts = false;
        merged.repeat_expressions = false;
        merged.shuangwen_metrics = false;
        merged.history_checks = false;

        for m in manifests {
            if let Ok(Some(rules)) = ProfileLoader::load_rules(root, &m.id) {
                if rules.extractors.people {
                    merged.people = true;
                }
                if rules.extractors.places {
                    merged.places = true;
                }
                if rules.extractors.items {
                    merged.items = true;
                }
                if rules.extractors.foreshadows {
                    merged.foreshadows = true;
                }
                if rules.extractors.eye_color_conflicts {
                    merged.eye_color_conflicts = true;
                }
                if rules.extractors.repeat_expressions {
                    merged.repeat_expressions = true;
                }
                if rules.extractors.shuangwen_metrics {
                    merged.shuangwen_metrics = true;
                }
                if rules.extractors.history_checks {
                    merged.history_checks = true;
                }
            }
        }

        Ok(Self { extractors: merged })
    }
}
