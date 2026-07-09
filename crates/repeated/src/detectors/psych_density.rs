use crate::Detector;
use crate::types::{ChunkInfo, RepeatedIssue};

pub struct PsychDensity;

impl Default for PsychDensity {
    fn default() -> Self {
        Self
    }
}

impl Detector for PsychDensity {
    fn id(&self) -> &'static str {
        "psych_density"
    }

    fn detect(&self, _chunks: &[ChunkInfo]) -> Vec<RepeatedIssue> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    // Tests added in Task 6
}
