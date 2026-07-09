#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ChunkInfo {
    pub document_id: String,
    pub chunk_id: String,
    pub document_path: String,
    pub chunk_index: i64,
    pub title: String,
    pub content: String,
    pub start_offset: i64,
    pub end_offset: i64,
}

#[derive(Debug, Clone)]
pub struct NarrativeNodeCandidate {
    pub node_type: String,
    pub name: String,
    pub aliases: Vec<String>,
    pub occurrence_count: i64,
    pub first_chunk_id: String,
    pub latest_chunk_id: String,
    pub confidence: i64,
}

#[derive(Debug, Clone)]
pub struct ForeshadowCandidate {
    pub title: String,
    pub foreshadow_type: String,
    pub first_chunk_id: String,
    pub latest_chunk_id: String,
    pub risk_level: String,
    pub evidence: String,
    pub related_nodes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct IssueCandidate {
    pub issue_type: String,
    pub severity: String,
    pub title: String,
    pub description: String,
    pub evidence_json: String,
    pub suggested_actions_json: String,
}

#[derive(Debug, Clone)]
pub enum Extraction {
    Candidate(NarrativeNodeCandidate),
    Foreshadow(ForeshadowCandidate),
    Issue(IssueCandidate),
}

pub trait Extractor {
    fn extract(&self, chunks: &[ChunkInfo]) -> Vec<Extraction>;
}
