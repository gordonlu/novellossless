use novellossless_storage::{NewChunk, ProjectChunk};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkDiff {
    pub added: Vec<ChunkDiffEntry>,
    pub removed: Vec<ChunkDiffEntry>,
    pub modified: Vec<ModifiedEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkDiffEntry {
    pub index: i64,
    pub title: String,
    pub hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModifiedEntry {
    pub index: i64,
    pub old_title: String,
    pub new_title: String,
    pub old_hash: String,
    pub new_hash: String,
}

pub fn diff_chunks(old_chunks: &[ProjectChunk], new_chunks: &[NewChunk]) -> ChunkDiff {
    let old_by_idx: HashMap<i64, &ProjectChunk> =
        old_chunks.iter().map(|c| (c.chunk_index, c)).collect();
    let new_by_idx: HashMap<i64, &NewChunk> =
        new_chunks.iter().map(|c| (c.chunk_index, c)).collect();

    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut modified = Vec::new();

    for (&idx, nc) in &new_by_idx {
        match old_by_idx.get(&idx) {
            None => added.push(ChunkDiffEntry {
                index: idx,
                title: nc.title.clone(),
                hash: nc.content_hash.clone(),
            }),
            Some(oc) if oc.title != nc.title || oc.content_hash != nc.content_hash => {
                modified.push(ModifiedEntry {
                    index: idx,
                    old_title: oc.title.clone(),
                    new_title: nc.title.clone(),
                    old_hash: oc.content_hash.clone(),
                    new_hash: nc.content_hash.clone(),
                });
            }
            _ => {}
        }
    }

    for (&idx, oc) in &old_by_idx {
        if !new_by_idx.contains_key(&idx) {
            removed.push(ChunkDiffEntry {
                index: idx,
                title: oc.title.clone(),
                hash: oc.content_hash.clone(),
            });
        }
    }

    ChunkDiff {
        added,
        removed,
        modified,
    }
}
