use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher as NotifyWatcher};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const SUPPORTED: &[&str] = &["txt", "md", "markdown"];
const DEBOUNCE_MS: u64 = 800;

pub struct FileWatcher {
    project_id: String,
    root: PathBuf,
    watcher: Option<RecommendedWatcher>,
    running: Arc<Mutex<bool>>,
}

impl FileWatcher {
    pub fn start<F>(project_id: &str, root: &Path, on_change: F) -> Result<Self, String>
    where
        F: Fn(String, PathBuf) + Send + 'static,
    {
        let (tx, rx) = mpsc::channel::<Result<Event, notify::Error>>();
        use notify::Config;

        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                let _ = tx.send(res);
            },
            Config::default(),
        )
        .map_err(|e| format!("failed to create watcher: {e}"))?;

        watcher.watch(root, RecursiveMode::Recursive)
            .map_err(|e| format!("failed to watch {root:?}: {e}"))?;

        let running = Arc::new(Mutex::new(true));
        let r = running.clone();
        let pid = project_id.to_string();

        thread::spawn(move || {
            let mut pending: HashSet<PathBuf> = HashSet::new();
            let mut last_event = Instant::now();

            loop {
                if !*r.lock().unwrap() { break; }
                match rx.recv_timeout(Duration::from_millis(200)) {
                    Ok(Ok(event)) => {
                        for path in &event.paths {
                            if is_supported(path) {
                                pending.insert(path.clone());
                                last_event = Instant::now();
                            }
                        }
                    }
                    Ok(Err(_)) => {}
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        if !pending.is_empty() && last_event.elapsed() >= Duration::from_millis(DEBOUNCE_MS) {
                            let batch: Vec<PathBuf> = pending.drain().collect();
                            for path in batch {
                                on_change(pid.clone(), path);
                            }
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
        });

        Ok(Self { project_id: project_id.to_string(), root: root.to_path_buf(), watcher: Some(watcher), running })
    }

    pub fn stop(&mut self) {
        if let Ok(mut r) = self.running.lock() {
            *r = false;
        }
        if let Some(mut w) = self.watcher.take() {
            let _ = w.unwatch(&self.root);
            drop(w);
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.lock().map(|r| *r).unwrap_or(false)
    }
}

fn is_supported(path: &Path) -> bool {
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        if name.starts_with('.') || name.starts_with('~') || name.ends_with('~') || name.contains(".swp") {
            return false;
        }
    }
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| SUPPORTED.contains(&e))
}
