mod similar_paragraphs;
mod high_freq_expressions;
mod repeated_actions;
mod dialogue_patterns;
mod psych_density;

pub use similar_paragraphs::SimilarParagraphs;
pub use high_freq_expressions::HighFreqExpressions;
pub use repeated_actions::RepeatedActions;
pub use dialogue_patterns::DialoguePatterns;
pub use psych_density::PsychDensity;

use crate::types::{ChunkInfo, RepeatedIssue};

pub trait Detector {
    fn id(&self) -> &'static str;
    fn detect(&self, chunks: &[ChunkInfo]) -> Vec<RepeatedIssue>;
}
