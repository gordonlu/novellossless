use std::path::Path;

pub use novellossless_profiles::{
    ExtractorRules, IssueEmitter, KnowledgePackIndex, KnowledgePackLoader, MetricRegistry,
    PeopleConfig, ProfileLoader, ProfileManifest, ProfileRules,
};

#[derive(Debug, Clone)]
pub struct AnalysisRules {
    pub extractors: ExtractorRules,
    pub people: PeopleConfig,
}

pub fn load_analysis_rules(profiles_root: &Path) -> AnalysisRules {
    let rules = ProfileLoader::load_rules(profiles_root, "common_longform")
        .ok()
        .flatten()
        .unwrap_or_default();
    AnalysisRules {
        extractors: rules.extractors,
        people: rules.people,
    }
}
