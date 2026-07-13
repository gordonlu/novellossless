use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use novellossless_core::NovelCore;

#[derive(Debug, Parser)]
#[command(name = "novellossless")]
#[command(about = "Local-first novel memory and continuity assistant")]
struct Cli {
    #[arg(long, default_value = "novellossless.db")]
    db: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Init,
    Import {
        #[arg(long)]
        name: String,
        #[arg(long)]
        path: PathBuf,
    },
    Scan {
        #[arg(long)]
        project_id: String,
    },
    IncrementalScan {
        #[arg(long)]
        project_id: String,
    },
    Search {
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        query: String,
        #[arg(long, default_value_t = 10)]
        limit: i64,
    },
    Candidates {
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        kind: Option<String>,
        #[arg(long, default_value_t = 20)]
        limit: i64,
    },
    Foreshadows {
        #[arg(long)]
        project_id: String,
        #[arg(long, default_value_t = 20)]
        limit: i64,
    },
    Issues {
        #[arg(long)]
        project_id: String,
        #[arg(long, default_value_t = 20)]
        limit: i64,
    },
    Context {
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        query: String,
        #[arg(long, default_value_t = 10)]
        limit: i64,
    },
    Profiles {
        #[arg(long)]
        project_id: Option<String>,
    },
    Tasks {
        #[arg(long)]
        project_id: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let core = NovelCore::open(&cli.db)?;

    match cli.command {
        Command::Init => {
            println!("initialized database: {}", cli.db.display());
        }
        Command::Import { name, path } => {
            let project = core.import_project(&name, &path)?;
            println!("project_id={}", project.id);
            println!("root_path={}", project.root_path);
        }
        Command::Scan { project_id } => {
            let report = core.scan_project(&project_id)?;
            println!("project_id={}", report.project_id);
            println!("scanned_documents={}", report.scanned_documents);
            println!("skipped_files={}", report.skipped_files);
            println!("document_count={}", report.summary.document_count);
            println!("chunk_count={}", report.summary.chunk_count);
            println!("total_words={}", report.summary.total_words);
            println!("person_candidates={}", report.analysis.person_candidates);
            println!("place_candidates={}", report.analysis.place_candidates);
            println!("item_candidates={}", report.analysis.item_candidates);
            println!(
                "foreshadow_candidates={}",
                report.analysis.foreshadow_candidates
            );
            println!("issue_count={}", report.analysis.issue_count);
        }
        Command::IncrementalScan { project_id } => {
            let report = core.incremental_scan(&project_id)?;
            println!("{} 增量扫描完成：", report.project_id);
            println!("  已扫描: {}", report.scanned_documents);
            println!("  新建: {}", report.created);
            println!("  修改: {}", report.modified);
            println!("  未变: {}", report.unchanged);
            println!("  删除: {}", report.deleted);
            println!("  失败: {}", report.failed);
        }
        Command::Search {
            project_id,
            query,
            limit,
        } => {
            for hit in core.search(&project_id, &query, limit)? {
                println!(
                    "{} | {} | {}:{}-{} | {}",
                    hit.title,
                    hit.document_path,
                    hit.chunk_index + 1,
                    hit.start_offset,
                    hit.end_offset,
                    hit.snippet
                );
            }
        }
        Command::Candidates {
            project_id,
            kind,
            limit,
        } => {
            for candidate in core.list_candidates(&project_id, kind.as_deref(), limit)? {
                println!(
                    "{} | {} | count={} | status={} | source={} {}",
                    candidate.node_type,
                    candidate.name,
                    candidate.occurrence_count,
                    candidate.status,
                    candidate.source_path,
                    candidate.source_title
                );
            }
        }
        Command::Foreshadows { project_id, limit } => {
            for item in core.list_foreshadows(&project_id, limit)? {
                println!(
                    "{} | {} | status={} | source={} {} | {}",
                    item.risk_level,
                    item.title,
                    item.status,
                    item.source_path,
                    item.source_title,
                    item.evidence
                );
            }
        }
        Command::Issues { project_id, limit } => {
            for issue in core.list_issues(&project_id, limit, None)? {
                println!(
                    "{} | {} | status={} | {}",
                    issue.severity, issue.title, issue.status, issue.description
                );
            }
        }
        Command::Context {
            project_id,
            query,
            limit,
        } => {
            let pack = core.build_context_pack(&project_id, &query, limit)?;
            println!("{}", pack.content);
        }
        Command::Tasks { project_id } => {
            for task in core.list_tasks(&project_id)? {
                println!(
                    "{} | {} | {} | {}",
                    task.priority, task.title, task.status, task.created_at
                );
            }
        }
        Command::Profiles { project_id } => {
            let available = core.get_available_profiles()?;
            println!("可用模式包 ({}):", available.len());
            for p in &available {
                println!("  {} | {} v{}", p.id, p.name, p.version);
                println!("    {}", p.description);
                if let Some(ref pid) = project_id {
                    let enabled = core.get_enabled_profiles(pid)?;
                    let is_enabled = enabled.contains(&p.id);
                    println!(
                        "    状态: {}",
                        if is_enabled {
                            "已启用 ✓"
                        } else {
                            "未启用"
                        }
                    );
                }
                if !p.metrics.enabled.is_empty() {
                    println!("    指标: {}", p.metrics.enabled.join(", "));
                }
                if !p.checks.enabled.is_empty() {
                    println!("    检查: {}", p.checks.enabled.join(", "));
                }
                println!();
            }
        }
    }

    Ok(())
}
