use anyhow::Result;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FsEvent {
    RequestChanged(String),     // rel_path of request e.g. "auth/login.yaml"
    RequestDeleted(String),     // rel_path of request
    EnvironmentChanged(String), // env name
    TreeChanged,                // collections folder structure or workspace.yaml
}

pub struct WorkspaceWatcher {
    _watcher: RecommendedWatcher,
}

pub fn start_workspace_watcher(
    root_dir: &Path,
    suppressed_paths: Arc<Mutex<HashSet<PathBuf>>>,
    event_sender: UnboundedSender<FsEvent>,
) -> Result<WorkspaceWatcher> {
    let (raw_tx, mut raw_rx) = unbounded_channel::<Event>();

    let mut watcher = RecommendedWatcher::new(
        move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                let _ = raw_tx.send(event);
            }
        },
        Config::default(),
    )?;

    watcher.watch(root_dir, RecursiveMode::Recursive)?;

    let root_buf = root_dir.to_path_buf();
    tokio::spawn(async move {
        let mut debounce_timer = tokio::time::interval(Duration::from_millis(200));
        let mut pending_events: HashSet<FsEvent> = HashSet::new();

        loop {
            tokio::select! {
                Some(event) = raw_rx.recv() => {
                    for path in &event.paths {
                        // Check if path is in suppressed set
                        if let Ok(set) = suppressed_paths.lock() {
                            if set.contains(path) {
                                continue;
                            }
                        }

                        let Ok(rel) = path.strip_prefix(&root_buf) else {
                            continue;
                        };
                        let rel_str = rel.to_string_lossy().replace('\\', "/");

                        // Ignore internal / non-collection files
                        if rel_str.starts_with(".git")
                            || rel_str.starts_with(".local")
                            || rel_str.ends_with(".loc")
                            || rel_str.ends_with(".tmp")
                            || rel_str.ends_with(".bak")
                        {
                            continue;
                        }

                        if rel_str == "workspace.yaml" {
                            pending_events.insert(FsEvent::TreeChanged);
                        } else if rel_str.starts_with("environments/") && rel_str.ends_with(".yaml") {
                            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                                pending_events.insert(FsEvent::EnvironmentChanged(stem.to_string()));
                            }
                        } else if rel_str.starts_with("collections/") {
                            let sub_rel = rel_str.trim_start_matches("collections/");
                            if sub_rel.ends_with("_folder.yaml") {
                                pending_events.insert(FsEvent::TreeChanged);
                            } else if sub_rel.ends_with(".yaml") {
                                match event.kind {
                                    EventKind::Remove(_) => {
                                        pending_events.insert(FsEvent::RequestDeleted(sub_rel.to_string()));
                                    }
                                    _ => {
                                        pending_events.insert(FsEvent::RequestChanged(sub_rel.to_string()));
                                    }
                                }
                            } else if path.is_dir() {
                                pending_events.insert(FsEvent::TreeChanged);
                            }
                        }
                    }
                }
                _ = debounce_timer.tick() => {
                    if !pending_events.is_empty() {
                        for ev in pending_events.drain() {
                            let _ = event_sender.send(ev);
                        }
                    }
                }
            }
        }
    });

    Ok(WorkspaceWatcher { _watcher: watcher })
}
