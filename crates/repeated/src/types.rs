#[derive(Debug, Clone)]
pub struct ChunkInfo {
    pub chunk_id: String,
    pub document_id: String,
    pub document_path: String,
    pub chapter_title: String,
    pub chunk_index: i64,
    pub content: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RepeatedIssue {
    pub issue_type: String,
    pub severity: String,
    pub title: String,
    pub description: String,
    pub evidence: Vec<EvidenceItem>,
    pub suggested_action: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct EvidenceItem {
    pub chunk_id: String,
    pub chapter_title: String,
    pub document_path: String,
    pub snippet: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_count: Option<u32>,
}
