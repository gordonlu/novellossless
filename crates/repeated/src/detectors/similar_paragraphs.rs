use crate::types::{ChunkInfo, RepeatedIssue};
use crate::Detector;

pub struct SimilarParagraphs;

impl Default for SimilarParagraphs {
    fn default() -> Self {
        Self
    }
}

impl Detector for SimilarParagraphs {
    fn id(&self) -> &'static str {
        "similar_paragraphs"
    }

    fn detect(&self, _chunks: &[ChunkInfo]) -> Vec<RepeatedIssue> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    // Tests added in Task 2
}
