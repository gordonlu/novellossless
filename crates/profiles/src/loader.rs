use crate::manifest::*;
use anyhow::{Context, Result};
use std::path::Path;

pub struct ProfileLoader;

impl ProfileLoader {
    pub fn load_all(profiles_root: &Path) -> Result<Vec<ProfileManifest>> {
        let mut manifests = Vec::new();
        let read_dir = match std::fs::read_dir(profiles_root) {
            Ok(d) => d,
            Err(_) => return Ok(manifests),
        };
        for entry in read_dir {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let profile_toml = entry.path().join("profile.toml");
            if !profile_toml.exists() {
                continue;
            }
            let content = std::fs::read_to_string(&profile_toml)
                .with_context(|| format!("failed to read {}", profile_toml.display()))?;
            match toml::from_str::<ProfileManifest>(&content) {
                Ok(mut m) => {
                    if m.id.is_empty() {
                        m.id = entry.file_name().to_string_lossy().to_string();
                    }
                    manifests.push(m);
                }
                Err(e) => {
                    log_warn(&format!("skipping {}: {e}", entry.path().display()));
                }
            }
        }
        manifests.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(manifests)
    }

    pub fn load_rules(profiles_root: &Path, profile_id: &str) -> Result<Option<ProfileRules>> {
        let path = profiles_root.join(profile_id).join("rules.toml");
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&path)?;
        toml::from_str(&content)
            .map(Some)
            .map_err(|e| anyhow::anyhow!("failed to parse {}: {e}", path.display()))
    }

    pub fn load_metrics_toml(
        profiles_root: &Path,
        profile_id: &str,
    ) -> Result<Option<MetricsToml>> {
        let path = profiles_root.join(profile_id).join("metrics.toml");
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&path)?;
        toml::from_str(&content)
            .map(Some)
            .map_err(|e| anyhow::anyhow!("failed to parse {}: {e}", path.display()))
    }
}

fn log_warn(msg: &str) {
    eprintln!("warning (profiles): {msg}");
}
