use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ProfileConfig {
    pub id: String,
    pub name: String,
    pub rules: ProfileRules,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ProfileRules {
    pub chapter_recognition: bool,
    pub full_text_search: bool,
    pub evidence_required: bool,
    pub auto_modify_source: bool,
}

impl Default for ProfileRules {
    fn default() -> Self {
        Self {
            chapter_recognition: true,
            full_text_search: true,
            evidence_required: true,
            auto_modify_source: false,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ExtractorRules {
    pub people: bool,
    pub places: bool,
    pub items: bool,
    pub foreshadows: bool,
    pub eye_color_conflicts: bool,
    pub repeat_expressions: bool,
}

impl Default for ExtractorRules {
    fn default() -> Self {
        Self {
            people: true,
            places: true,
            items: true,
            foreshadows: true,
            eye_color_conflicts: true,
            repeat_expressions: true,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct PeopleConfig {
    pub min_name_length: u32,
    pub max_name_length: u32,
    pub enable_alias_detection: bool,
}

impl Default for PeopleConfig {
    fn default() -> Self {
        Self {
            min_name_length: 2,
            max_name_length: 4,
            enable_alias_detection: true,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct AnalysisRules {
    pub extractors: ExtractorRules,
    pub people: PeopleConfig,
}

impl Default for AnalysisRules {
    fn default() -> Self {
        Self {
            extractors: ExtractorRules::default(),
            people: PeopleConfig::default(),
        }
    }
}

pub fn load_analysis_rules(profiles_root: &std::path::Path) -> AnalysisRules {
    let path = profiles_root.join("common_longform").join("rules.toml");
    if !path.exists() {
        return AnalysisRules::default();
    }
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return AnalysisRules::default(),
    };
    toml::from_str(&content).unwrap_or_default()
}
