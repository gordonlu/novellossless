use crate::types::{ChunkInfo, RepeatedIssue};
use crate::Detector;

pub struct DialoguePatterns;

impl Default for DialoguePatterns {
    fn default() -> Self {
        Self
    }
}

impl Detector for DialoguePatterns {
    fn id(&self) -> &'static str {
        "dialogue_patterns"
    }

    fn detect(&self, _chunks: &[ChunkInfo]) -> Vec<RepeatedIssue> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    // Tests added in Task 5
}
