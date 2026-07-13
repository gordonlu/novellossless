pub mod provider;

use anyhow::Result;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ExtractedRule {
    pub name: String,
    pub description: String,
    pub rule_type: String,
    pub keywords: Vec<String>,
    pub positive: bool,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct TimelineInsight {
    pub chunk_id: String,
    pub time_description: String,
    pub suggested_order: Option<i64>,
    pub is_flashback: bool,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ImpactInsight {
    pub summary: String,
    pub affected_areas: Vec<String>,
}

pub trait AiProvider: Send + Sync {
    fn extract_rules(&self, chunks: &[&str]) -> Result<Vec<ExtractedRule>> {
        let _ = chunks;
        Ok(Vec::new())
    }
    fn analyze_timeline(&self, chunks: &[&str]) -> Result<Vec<TimelineInsight>> {
        let _ = chunks;
        Ok(Vec::new())
    }
    fn analyze_impact(&self, old: &[&str], new: &[&str], diff_desc: &str) -> Result<ImpactInsight> {
        let _ = (old, new, diff_desc);
        Ok(ImpactInsight {
            summary: "AI impact analysis not configured".to_string(),
            affected_areas: Vec::new(),
        })
    }
}

pub struct NoopProvider;

impl AiProvider for NoopProvider {}

impl AiProvider for provider::OpenAiProvider {
    fn extract_rules(&self, chunks: &[&str]) -> Result<Vec<ExtractedRule>> {
        let text = chunks.join("\n---\n");
        let system = r#"你是一个长篇小说一致性分析助手。你的任务是从正文片段中提取"重复出现规则"——即小说中反复使用的人物特征、地点描述、物件特征。

规则要求：
- name: 简短规则名称（中文，如"林澈的剑"、"青石巷的灯笼"）
- description: 规则说明，描述该规则所指的反复出现的特征（50字以内）
- rule_type: 只能取 "person"（人物特征）、"place"（地点特征）、"object"（物件特征）
- keywords: 与该规则相关的关键词列表，2-5个
- positive: true 表示正面提及/存在，false 表示负面/缺失

输出 strict JSON 数组，schema:
[{"name": string, "description": string, "rule_type": "person"|"place"|"object", "keywords": string[], "positive": bool}]

注意事项：
- 只抽取明显重复出现的特征，不要针对单次出现的细节
- 每条规则必须有独立的文本依据
- 如无匹配规则，返回空数组 []
- 只输出 JSON，不要额外说明"#;
        let response = self.chat_completion(system, &text)?;
        let cleaned = strip_json_fence(&response);
        serde_json::from_str(cleaned).or_else(|_| Ok(Vec::new()))
    }

    fn analyze_timeline(&self, chunks: &[&str]) -> Result<Vec<TimelineInsight>> {
        let text = chunks.join("\n---\n");
        let system = r#"你是一个叙事时间线分析助手。你的任务是从段落中识别时间描述和倒叙。

每个段落按输入顺序编号（从1开始），输出时填入 chunk_id。

规则：
- chunk_id: 段落序号（字符串，如 "1", "2"）
- time_description: 该段落中出现的具体时间描述（如"三年前"、"次日清晨"、"同治年间"），如无明确时间则写"无明确时间"
- suggested_order: 如果该段落的时间点明显不按顺序（倒叙/插叙），给出建议的正确顺序编号；按时间先后排列的段落此项为 null
- is_flashback: true 如果该段落明显是倒叙/回忆场景，否则 false

输出 strict JSON 数组:
[{"chunk_id": string, "time_description": string, "suggested_order": number|null, "is_flashback": bool}]

注意事项：
- 只识别文本中明确出现的时间描述，不推测隐含时间
- 倒叙判断依据：明确的时间跳跃（如"三年前"）、场景切换标记（回忆口吻）
- 如无时间信息，返回空数组 []
- 只输出 JSON，不要额外说明"#;
        let response = self.chat_completion(system, &text)?;
        let cleaned = strip_json_fence(&response);
        serde_json::from_str(cleaned).or_else(|_| Ok(Vec::new()))
    }

    fn analyze_impact(&self, old: &[&str], new: &[&str], diff_desc: &str) -> Result<ImpactInsight> {
        let old_text = old.join("\n---\n");
        let new_text = new.join("\n---\n");
        let system = r#"你是一个长篇小说修订影响评估助手。你的任务是对比修改前后的文本，评估修改对小说整体产生的影响。

分析维度（在 affected_areas 中列出受影响的领域）：
- "情节推进"：事件发展、悬念设置是否改变
- "人物塑造"：人物形象、动机、关系是否改变
- "叙事节奏"：段落长短、信息密度是否变化
- "氛围基调"：情绪色彩、氛围是否变化
- "对话风格"：对白风格、语气语调是否变化

输出 strict JSON 对象:
{"summary": string, "affected_areas": string[]}

- summary: 100字以内的影响概要，概括修改带来的主要变化和潜在影响
- affected_areas: 上述维度中的受影响项列表，如 ["情节推进", "人物塑造"]

注意事项：
- 如果修改很小或只涉及措辞，summary 如实描述，affected_areas 可只保留最相关的1-2项
- 如果修改涉及多个维度，affected_areas 列出所有受影响的维度
- 只输出 JSON，不要额外说明"#;
        let user = format!(
            "修改说明：{}\n\n修改前：\n{}\n\n修改后：\n{}",
            diff_desc, old_text, new_text
        );
        let response = self.chat_completion(system, &user)?;
        let cleaned = strip_json_fence(&response);
        Ok(
            serde_json::from_str(cleaned).unwrap_or_else(|_| ImpactInsight {
                summary: response,
                affected_areas: Vec::new(),
            }),
        )
    }
}

fn strip_json_fence(text: &str) -> &str {
    text.trim()
        .strip_prefix("```json")
        .or_else(|| text.trim().strip_prefix("```"))
        .unwrap_or(text.trim())
        .strip_suffix("```")
        .unwrap_or(text.trim())
}

#[cfg(feature = "deeplossless-compat")]
pub struct DeeplosslessProvider;

#[cfg(feature = "deeplossless-compat")]
impl AiProvider for DeeplosslessProvider {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_provider_returns_empty() {
        let provider = NoopProvider;
        let rules = provider.extract_rules(&["test"]).unwrap();
        assert!(rules.is_empty());
        let insights = provider.analyze_timeline(&["test"]).unwrap();
        assert!(insights.is_empty());
        let impact = provider.analyze_impact(&["old"], &["new"], "diff").unwrap();
        assert!(!impact.summary.is_empty());
    }
}
