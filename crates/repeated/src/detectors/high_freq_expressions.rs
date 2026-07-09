use crate::types::{ChunkInfo, RepeatedIssue};
use crate::Detector;

pub struct HighFreqExpressions;

impl Default for HighFreqExpressions {
    fn default() -> Self {
        Self
    }
}

impl Detector for HighFreqExpressions {
    fn id(&self) -> &'static str {
        "high_freq_expressions"
    }

    fn detect(&self, _chunks: &[ChunkInfo]) -> Vec<RepeatedIssue> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    // Tests added in Task 3
}
