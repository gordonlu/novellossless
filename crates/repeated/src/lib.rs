mod detectors;
pub mod types;

pub use detectors::Detector;
pub use types::{ChunkInfo, EvidenceItem, RepeatedIssue};

pub struct RepeatedDescriptionEngine {
    detectors: Vec<Box<dyn Detector>>,
}

impl RepeatedDescriptionEngine {
    pub fn new() -> Self {
        Self {
            detectors: Vec::new(),
        }
    }

    pub fn with(mut self, detector: Box<dyn Detector>) -> Self {
        self.detectors.push(detector);
        self
    }

    pub fn detect(&self, chunks: &[ChunkInfo]) -> Vec<RepeatedIssue> {
        if chunks.is_empty() {
            return Vec::new();
        }
        let mut results = Vec::new();
        for detector in &self.detectors {
            results.extend(detector.detect(chunks));
        }
        results
    }
}

impl Default for RepeatedDescriptionEngine {
    fn default() -> Self {
        Self {
            detectors: vec![
                Box::new(detectors::SimilarParagraphs::default()),
                Box::new(detectors::HighFreqExpressions::default()),
                Box::new(detectors::RepeatedActions::default()),
                Box::new(detectors::DialoguePatterns::default()),
                Box::new(detectors::PsychDensity::default()),
            ],
        }
    }
}
