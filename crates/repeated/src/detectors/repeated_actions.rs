use crate::types::{ChunkInfo, RepeatedIssue};
use crate::Detector;

pub struct RepeatedActions;

impl Default for RepeatedActions {
    fn default() -> Self {
        Self
    }
}

impl Detector for RepeatedActions {
    fn id(&self) -> &'static str {
        "repeated_actions"
    }

    fn detect(&self, _chunks: &[ChunkInfo]) -> Vec<RepeatedIssue> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    // Tests added in Task 4
}
