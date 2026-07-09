use anyhow::Result;

#[derive(Debug, Clone)]
pub struct ExtractedRule {
    pub name: String,
    pub description: String,
    pub rule_type: String,
    pub keywords: Vec<String>,
    pub positive: bool,
}

#[derive(Debug, Clone)]
pub struct TimelineInsight {
    pub chunk_id: String,
    pub time_description: String,
    pub suggested_order: Option<i64>,
    pub is_flashback: bool,
}

#[derive(Debug, Clone)]
pub struct ImpactInsight {
    pub summary: String,
    pub affected_areas: Vec<String>,
}

pub trait AiProvider: Send + Sync {
    fn extract_rules(&self, chunks: &[&str]) -> Result<Vec<ExtractedRule>> {
        let _ = chunks;
        Ok(Vec::new())
    }
    fn analyze_timeline(&self, chunks: &[&str]) -> Result<Vec<TimelineInsight>> {
        let _ = chunks;
        Ok(Vec::new())
    }
    fn analyze_impact(&self, old: &[&str], new: &[&str], diff_desc: &str) -> Result<ImpactInsight> {
        let _ = (old, new, diff_desc);
        Ok(ImpactInsight {
            summary: "AI impact analysis not configured".to_string(),
            affected_areas: Vec::new(),
        })
    }
}

pub struct NoopProvider;

impl AiProvider for NoopProvider {}

#[cfg(feature = "deeplossless-compat")]
pub struct DeeplosslessProvider;

#[cfg(feature = "deeplossless-compat")]
impl AiProvider for DeeplosslessProvider {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_provider_returns_empty() {
        let provider = NoopProvider;
        let rules = provider.extract_rules(&["test"]).unwrap();
        assert!(rules.is_empty());
        let insights = provider.analyze_timeline(&["test"]).unwrap();
        assert!(insights.is_empty());
        let impact = provider.analyze_impact(&["old"], &["new"], "diff").unwrap();
        assert!(!impact.summary.is_empty());
    }
}