use std::fs;
use std::path::Path;
use std::sync::OnceLock;

use anyhow::{Context, Result};
use encoding_rs::{GB18030, UTF_8};
use regex::Regex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextDocument {
    pub content: String,
    pub encoding: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedDocument {
    pub title: String,
    pub content: String,
    pub encoding: String,
    pub chapters: Vec<Chapter>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chapter {
    pub index: usize,
    pub title: String,
    pub start_offset: usize,
    pub end_offset: usize,
    pub content: String,
}

pub fn read_text_file(path: &Path) -> Result<TextDocument> {
    let bytes = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    decode_text(&bytes)
}

pub fn decode_text(bytes: &[u8]) -> Result<TextDocument> {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        let (content, _, had_errors) = UTF_8.decode(&bytes[3..]);
        if had_errors {
            anyhow::bail!("UTF-8 BOM file contains invalid UTF-8");
        }

        return Ok(TextDocument {
            content: content.into_owned(),
            encoding: "utf-8-bom".to_string(),
        });
    }

    if let Ok(content) = std::str::from_utf8(bytes) {
        return Ok(TextDocument {
            content: content.to_string(),
            encoding: "utf-8".to_string(),
        });
    }

    let (content, _, had_errors) = GB18030.decode(bytes);
    if had_errors {
        anyhow::bail!("file is neither valid UTF-8 nor GB18030 text");
    }

    Ok(TextDocument {
        content: content.into_owned(),
        encoding: "gb18030".to_string(),
    })
}

pub fn read_docx_file(path: &Path) -> Result<TextDocument> {
    let docx_file = docx_rust::DocxFile::from_file(path)
        .with_context(|| format!("failed to open docx {}", path.display()))?;
    let docx = docx_file
        .parse()
        .with_context(|| format!("failed to parse docx structure in {}", path.display()))?;

    let mut paragraphs: Vec<String> = Vec::new();
    for content in &docx.document.body.content {
        if let docx_rust::document::BodyContent::Paragraph(para) = content {
            let text = para.text().trim().to_string();
            if !text.is_empty() {
                paragraphs.push(text);
            }
        }
    }

    let content = paragraphs.join("\n\n");
    if content.is_empty() {
        anyhow::bail!("no text content found in docx {}", path.display());
    }

    Ok(TextDocument {
        content,
        encoding: "utf-8".to_string(),
    })
}

pub fn parse_document(path: &Path) -> Result<ParsedDocument> {
    let is_docx = path
        .extension()
        .and_then(|v| v.to_str())
        .is_some_and(|v| v.eq_ignore_ascii_case("docx"));

    let text = if is_docx {
        read_docx_file(path)?
    } else {
        read_text_file(path)?
    };

    let title = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("未命名文档")
        .to_string();
    let chapters = split_chapters(&text.content, &title);

    Ok(ParsedDocument {
        title,
        content: text.content,
        encoding: text.encoding,
        chapters,
    })
}

pub fn split_chapters(content: &str, fallback_title: &str) -> Vec<Chapter> {
    let mut starts = Vec::new();
    let mut offset = 0;

    for line in content.split_inclusive('\n') {
        let title = line.trim();
        if is_chapter_title(title) {
            starts.push((offset, title.to_string()));
        }
        offset += line.len();
    }

    let trailing = if content.ends_with('\n') { 0 } else { 1 };
    if trailing == 1 {
        let last_line_start = content.rfind('\n').map(|index| index + 1).unwrap_or(0);
        let title = content[last_line_start..].trim();
        if is_chapter_title(title) && !starts.iter().any(|(start, _)| *start == last_line_start) {
            starts.push((last_line_start, title.to_string()));
        }
    }

    if starts.is_empty() {
        return vec![Chapter {
            index: 0,
            title: fallback_title.to_string(),
            start_offset: 0,
            end_offset: content.len(),
            content: content.to_string(),
        }];
    }

    starts
        .iter()
        .enumerate()
        .map(|(index, (start, title))| {
            let end = starts
                .get(index + 1)
                .map(|(next_start, _)| *next_start)
                .unwrap_or(content.len());
            Chapter {
                index,
                title: title.clone(),
                start_offset: *start,
                end_offset: end,
                content: content[*start..end].trim().to_string(),
            }
        })
        .collect()
}

fn is_chapter_title(line: &str) -> bool {
    static TITLE_RE: OnceLock<Regex> = OnceLock::new();
    let pattern = TITLE_RE.get_or_init(|| {
        Regex::new(
            r"(?ix)
            ^
            (
                第[\p{Han}0-9０-９〇零一二三四五六七八九十百千万两]+[章节回卷部].*
              | 卷[\p{Han}0-9０-９〇零一二三四五六七八九十百千万两]+.*
              | 序章
              | 楔子
              | 番外.*
              | chapter\s+[0-9]+.*
              | [0-9０-９]{3,}\s*[-_－—].*
            )
            $
            ",
        )
        .expect("chapter title regex is valid")
    });

    pattern.is_match(line)
}

fn make_docx(paragraphs: &[&str]) -> Vec<u8> {
    use docx_rust::Docx;
    use docx_rust::document::Paragraph;
    let mut docx = Docx::default();
    for text in paragraphs {
        let para = Paragraph::default().push_text(*text);
        docx.document.body.push(para);
    }
    let mut buf = std::io::Cursor::new(Vec::new());
    docx.write(&mut buf).unwrap();
    buf.into_inner()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_docx_content() {
        let docx = make_docx(&["第一章 雨夜", "林澈推开木门。"]);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.docx");
        std::fs::write(&path, docx).unwrap();
        let doc = parse_document(&path).unwrap();
        assert!(doc.content.contains("第一章 雨夜"));
        assert!(doc.content.contains("林澈推开木门"));
        assert_eq!(doc.encoding, "utf-8");
    }

    #[test]
    fn parses_docx_with_chapters() {
        let docx = make_docx(&["第一章 雨夜", "林澈醒来。", "第二章 钟声", "钟声响了。"]);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.docx");
        std::fs::write(&path, docx).unwrap();
        let doc = parse_document(&path).unwrap();
        assert_eq!(doc.chapters.len(), 2);
        assert_eq!(doc.chapters[0].title, "第一章 雨夜");
        assert_eq!(doc.chapters[1].title, "第二章 钟声");
    }

    #[test]
    fn parses_docx_preserves_paragraphs() {
        let docx = make_docx(&["第一段。", "第二段。", "第三段。"]);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.docx");
        std::fs::write(&path, docx).unwrap();
        let doc = parse_document(&path).unwrap();
        assert!(doc.content.contains("第一段。"));
        assert!(doc.content.contains("第二段。"));
        assert!(doc.content.contains("第三段。"));
        // Paragraphs should be separated by double newlines
        assert!(doc.content.contains("第一段。\n\n第二段。"));
    }

    #[test]
    fn docx_invalid_zip_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.docx");
        std::fs::write(&path, "not a zip file").unwrap();
        let result = parse_document(&path);
        assert!(result.is_err());
    }

    #[test]
    fn splits_chinese_chapter_titles() {
        let content = "序章\n雨声落下\n第一章 雨夜\n林澈醒来\n第十二章 回声\n钟声响起";
        let chapters = split_chapters(content, "fallback");

        assert_eq!(chapters.len(), 3);
        assert_eq!(chapters[0].title, "序章");
        assert_eq!(chapters[1].title, "第一章 雨夜");
        assert_eq!(chapters[2].title, "第十二章 回声");
        assert!(chapters[1].content.contains("林澈醒来"));
    }

    #[test]
    fn uses_single_document_when_no_title_matches() {
        let chapters = split_chapters("林澈走进雨里。", "雨巷钟声");

        assert_eq!(chapters.len(), 1);
        assert_eq!(chapters[0].title, "雨巷钟声");
        assert_eq!(chapters[0].start_offset, 0);
    }

    #[test]
    fn decodes_gb18030_when_utf8_fails() {
        let bytes = b"\xC1\xD6\xB3\xBA";
        let document = decode_text(bytes).expect("gb18030 should decode");

        assert_eq!(document.content, "林澈");
        assert_eq!(document.encoding, "gb18030");
    }
}
