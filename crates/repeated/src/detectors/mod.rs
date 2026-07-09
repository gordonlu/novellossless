mod dialogue_patterns;
mod high_freq_expressions;
mod psych_density;
mod repeated_actions;
mod similar_paragraphs;

pub use dialogue_patterns::DialoguePatterns;
pub use high_freq_expressions::HighFreqExpressions;
pub use psych_density::PsychDensity;
pub use repeated_actions::RepeatedActions;
pub use similar_paragraphs::SimilarParagraphs;

use crate::types::{ChunkInfo, RepeatedIssue};

pub trait Detector {
    fn id(&self) -> &'static str;
    fn detect(&self, chunks: &[ChunkInfo]) -> Vec<RepeatedIssue>;
}
