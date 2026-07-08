use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ProfileManifest {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub enabled_by_default: Option<bool>,

    #[serde(default)]
    pub entities: EntityTypes,
    #[serde(default)]
    pub facts: FactTypes,
    #[serde(default)]
    pub events: EventTypes,
    #[serde(default)]
    pub metrics: MetricDefs,
    #[serde(default)]
    pub checks: CheckDefs,
    #[serde(default)]
    pub templates: TemplateDefs,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct EntityTypes {
    pub types: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct FactTypes {
    pub types: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct EventTypes {
    pub types: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct MetricDefs {
    pub enabled: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct CheckDefs {
    pub enabled: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct TemplateDefs {
    pub enabled: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ReportDefs {
    pub enabled: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ProfileRules {
    pub chapter_recognition: bool,
    pub full_text_search: bool,
    pub evidence_required: bool,
    pub auto_modify_source: bool,
    pub extractors: ExtractorRules,
    pub people: PeopleConfig,
}

impl Default for ProfileRules {
    fn default() -> Self {
        Self {
            chapter_recognition: true,
            full_text_search: true,
            evidence_required: true,
            auto_modify_source: false,
            extractors: ExtractorRules::default(),
            people: PeopleConfig::default(),
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
    #[serde(default)]
    pub shuangwen_metrics: bool,
    #[serde(default)]
    pub history_checks: bool,
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
            shuangwen_metrics: false,
            history_checks: false,
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

#[derive(Debug, Clone, Default, Deserialize)]
pub struct MetricsToml {
    #[serde(default)]
    pub metrics: Vec<MetricTomlEntry>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct MetricTomlEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default = "default_weight")]
    pub weight: f64,
}

fn default_weight() -> f64 {
    1.0
}

#[derive(Debug, Clone, Default)]
pub struct CheckDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub profile_id: String,
    pub severity: String,
}
