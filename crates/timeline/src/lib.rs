use anyhow::Result;
use novellossless_storage::{NewContinuityIssue, ProjectChunk, Storage, TimelineEvent};
use regex::Regex;
use serde_json::json;

fn parse_chinese_number(s: &str) -> Option<i64> {
    let digit: fn(char) -> Option<i64> = |c| match c {
        '零' => Some(0),
        '一' => Some(1),
        '二' | '两' => Some(2),
        '三' => Some(3),
        '四' => Some(4),
        '五' => Some(5),
        '六' => Some(6),
        '七' => Some(7),
        '八' => Some(8),
        '九' => Some(9),
        '十' => Some(10),
        '百' => Some(100),
        '千' => Some(1000),
        _ => None,
    };
    let chars: Vec<char> = s.chars().collect();
    if chars.is_empty() {
        return None;
    }
    if chars.len() == 1 {
        return digit(chars[0]);
    }
    // compound: e.g., 十一 -> 11, 二十三 -> 23, 一百二十 -> 120
    let mut total: i64 = 0;
    let mut cur: i64 = 0;
    for &c in &chars {
        if let Some(d) = digit(c) {
            if d >= 10 {
                total += if cur == 0 { d } else { cur * d };
                cur = 0;
            } else {
                cur = d;
            }
        } else {
            return None;
        }
    }
    Some(total + cur)
}

pub struct TimelineEngine;

impl TimelineEngine {
    pub fn extract(project_id: &str, chunks: &[ProjectChunk], storage: &Storage) -> Result<()> {
        storage.delete_project_timeline_events(project_id)?;

        let relative_re =
            Regex::new(r"(\d+|[一二三四五六七八九十百千万]+)(?:天|个?月|年)(?:[后之]?[后前])")?;
        let absolute_re = Regex::new(
            r"(天宝|贞观|开元|神龙|武德|乾元|大历|建中|贞元|元和|长庆|宝历|太和|开成|会昌|大中|咸通|乾符|广明|中和|光启|文德|龙纪|大顺|景福|乾宁|光化|天复|天祐|景德|祥符|天禧|乾兴|天圣|明道|景祐|宝元|康定|庆历|皇祐|至和|嘉祐|治平|熙宁|元丰|元祐|绍圣|元符|靖国|崇宁|大观|政和|重和|宣和|靖康|建炎|绍兴|隆兴|乾道|淳熙|绍熙|庆元|嘉泰|开禧|嘉定|宝庆|绍定|端平|嘉熙|淳祐|宝祐|开庆|景定|咸淳|德祐|景炎|祥兴)(\d+|[一二三四五六七八九十百千万]+)(?:载|年)?",
        )?;
        let flashback_re = Regex::new(r"(回忆起|想起|回想|那年|曾经|当时|那时|多年前|很久以前)")?;

        let mut cursor: i64 = 0;

        for chunk in chunks {
            cursor += 1;
            let mut time_expr = String::new();
            let mut estimated_order: Option<i64> = None;
            let mut is_flashback = false;

            if let Some(cap) = relative_re.captures(&chunk.content) {
                let raw = cap.get(1).map(|m| m.as_str()).unwrap_or("1");
                let n = raw
                    .parse::<i64>()
                    .ok()
                    .or_else(|| parse_chinese_number(raw));
                if let Some(n) = n {
                    estimated_order = Some(cursor + n);
                    time_expr = cap.get(0).map(|m| m.as_str()).unwrap_or("").to_string();
                }
            }

            if absolute_re.is_match(&chunk.content) {
                if let Some(cap) = absolute_re.captures(&chunk.content) {
                    time_expr = cap.get(0).map(|m| m.as_str()).unwrap_or("").to_string();
                    if let Some(num_str) = cap.get(2).map(|m| m.as_str()) {
                        if let Ok(n) = num_str.parse::<i64>() {
                            estimated_order = Some(n);
                        } else if let Some(n) = parse_chinese_number(num_str) {
                            estimated_order = Some(n);
                        }
                    }
                }
            }

            if flashback_re.is_match(&chunk.content) {
                is_flashback = true;
            }

            let event = TimelineEvent {
                id: uuid::Uuid::new_v4().to_string(),
                project_id: project_id.to_string(),
                chunk_id: chunk.chunk_id.clone(),
                chunk_index: chunk.chunk_index,
                document_path: chunk.document_path.clone(),
                title: chunk.title.clone(),
                order_index: cursor,
                time_expression: time_expr,
                estimated_order,
                participants_json: String::from("[]"),
                location: String::new(),
                is_flashback,
                confidence: 50,
            };
            storage.upsert_timeline_event(&event)?;
        }

        Ok(())
    }

    pub fn check(events: &[TimelineEvent], _chunks: &[ProjectChunk]) -> Vec<NewContinuityIssue> {
        let mut issues = Vec::new();

        // Same-person two-locations: check sequential events with same participant
        // at significantly different order_index without location change
        for (i, event) in events.iter().enumerate() {
            let participants: Vec<String> =
                serde_json::from_str(&event.participants_json).unwrap_or_default();
            if participants.is_empty() || event.location.is_empty() {
                continue;
            }

            if let Some(prev) = events.get(i.saturating_sub(1)) {
                let prev_participants: Vec<String> =
                    serde_json::from_str(&prev.participants_json).unwrap_or_default();
                let shared: Vec<&String> = participants
                    .iter()
                    .filter(|p| prev_participants.contains(p))
                    .collect();
                for p in shared {
                    if !prev.location.is_empty()
                        && prev.location != event.location
                        && (event.order_index - prev.order_index).abs() <= 2
                    {
                        issues.push(NewContinuityIssue {
                            issue_type: "time_anomaly".to_string(),
                            severity: "medium".to_string(),
                            title: format!("{} 短时间内出现在两个地点", p),
                            description: format!(
                                "{} 在 {} （{}）和 {} （{}）之间距离太近。",
                                p, prev.title, prev.location, event.title, event.location
                            ),
                            evidence_json: serde_json::to_string(&json!({
                                "person": p,
                                "location_a": prev.location,
                                "location_b": event.location,
                                "chapter_a": prev.title,
                                "chapter_b": event.title,
                            }))
                            .unwrap_or_default(),
                            suggested_actions_json: String::from(
                                r#"["确认是两个不同地点","检查时间跳跃","标记误报"]"#,
                            ),
                        });
                    }
                }
            }
        }

        issues
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use novellossless_storage::{NewChunk, NewDocument, ProjectChunk, Storage};

    fn seed_chunks(
        storage: &Storage,
        project_id: &str,
        chunks: &[ProjectChunk],
    ) -> Result<Vec<ProjectChunk>> {
        let doc = NewDocument {
            path: chunks[0].document_path.clone(),
            kind: "novel".into(),
            title: chunks[0].title.clone(),
            chapter_count: chunks.len() as i64,
            content_hash: "test_hash".into(),
            word_count: chunks.iter().map(|c| c.word_count).sum(),
            encoding: "utf-8".into(),
        };
        let new_chunks: Vec<NewChunk> = chunks
            .iter()
            .map(|c| NewChunk {
                chunk_index: c.chunk_index,
                title: c.title.clone(),
                start_offset: c.start_offset,
                end_offset: c.end_offset,
                content: c.content.clone(),
                content_hash: c.content_hash.clone(),
                word_count: c.word_count,
            })
            .collect();
        storage.upsert_document_with_chunks(project_id, &doc, &new_chunks)?;
        storage.project_chunks(project_id)
    }

    #[test]
    fn extracts_relative_time() -> Result<()> {
        let storage = Storage::open_memory()?;
        let project = storage.create_project("time_rel", "/tmp/time_rel")?;
        let input = vec![ProjectChunk {
            document_id: String::new(),
            chunk_id: String::new(),
            document_path: "001.txt".into(),
            chunk_index: 0,
            title: "第一章".into(),
            content: "三天后，他到了长安。".into(),
            start_offset: 0,
            end_offset: 12,
            word_count: 6,
            content_hash: "h1".into(),
        }];
        let chunks = seed_chunks(&storage, &project.id, &input)?;
        TimelineEngine::extract(&project.id, &chunks, &storage)?;
        let events = storage.list_timeline_events(&project.id)?;
        assert_eq!(events.len(), 1);
        assert!(events[0].time_expression.contains("三天"));
        assert_eq!(events[0].estimated_order, Some(4));
        Ok(())
    }

    #[test]
    fn extracts_absolute_year() -> Result<()> {
        let storage = Storage::open_memory()?;
        let project = storage.create_project("time_abs", "/tmp/time_abs")?;
        let input = vec![ProjectChunk {
            document_id: String::new(),
            chunk_id: String::new(),
            document_path: "001.txt".into(),
            chunk_index: 0,
            title: "第一章".into(),
            content: "贞观三年，长安城内一片繁华。".into(),
            start_offset: 0,
            end_offset: 16,
            word_count: 8,
            content_hash: "h1".into(),
        }];
        let chunks = seed_chunks(&storage, &project.id, &input)?;
        TimelineEngine::extract(&project.id, &chunks, &storage)?;
        let events = storage.list_timeline_events(&project.id)?;
        assert_eq!(events.len(), 1);
        assert!(events[0].time_expression.contains("贞观"));
        assert!(
            events[0].estimated_order.is_some(),
            "estimated_order should be set for absolute year '贞观三年'"
        );
        Ok(())
    }

    #[test]
    fn detects_flashback() -> Result<()> {
        let storage = Storage::open_memory()?;
        let project = storage.create_project("flash", "/tmp/flash")?;
        let input = vec![ProjectChunk {
            document_id: String::new(),
            chunk_id: String::new(),
            document_path: "001.txt".into(),
            chunk_index: 0,
            title: "第一章".into(),
            content: "林澈回忆起那年冬天的事情。".into(),
            start_offset: 0,
            end_offset: 16,
            word_count: 8,
            content_hash: "h1".into(),
        }];
        let chunks = seed_chunks(&storage, &project.id, &input)?;
        TimelineEngine::extract(&project.id, &chunks, &storage)?;
        let events = storage.list_timeline_events(&project.id)?;
        assert!(events[0].is_flashback);
        Ok(())
    }

    #[test]
    fn extract_empty_chunks() -> Result<()> {
        let storage = Storage::open_memory()?;
        let project = storage.create_project("empty_tl", "/tmp/empty_tl")?;
        TimelineEngine::extract(&project.id, &[], &storage)?;
        let events = storage.list_timeline_events(&project.id)?;
        assert!(events.is_empty());
        Ok(())
    }

    #[test]
    fn extract_no_time_expression() -> Result<()> {
        let storage = Storage::open_memory()?;
        let project = storage.create_project("no_time", "/tmp/no_time")?;
        let input = vec![ProjectChunk {
            document_id: String::new(),
            chunk_id: String::new(),
            document_path: "001.txt".into(),
            chunk_index: 0,
            title: "第一章".into(),
            content: "林澈走进了长安城。".into(),
            start_offset: 0,
            end_offset: 11,
            word_count: 5,
            content_hash: "h1".into(),
        }];
        let chunks = seed_chunks(&storage, &project.id, &input)?;
        TimelineEngine::extract(&project.id, &chunks, &storage)?;
        let events = storage.list_timeline_events(&project.id)?;
        assert_eq!(
            events.len(),
            1,
            "should create event even without time expression"
        );
        assert!(events[0].time_expression.is_empty());
        assert!(events[0].estimated_order.is_none());
        Ok(())
    }
}
