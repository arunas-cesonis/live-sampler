use crate::utils::FileStatus;

use notify::event::{CreateKind, ModifyKind};
use notify::{RecommendedWatcher, Watcher};
use std::path::{Path, PathBuf};
use std::time::Instant;

pub struct Source {
    pub(crate) text: String,
    pub(crate) time: Instant,
}

impl Source {
    pub fn new(text: String) -> Self {
        Self {
            text,
            time: Instant::now(),
        }
    }
}

pub enum SourceState {
    Empty,
    Loaded((PathBuf, Option<PathWatcher>, Source)),
    Error((PathBuf, Option<PathWatcher>, std::io::Error)),
}

impl SourceState {
    pub fn empty() -> Self {
        SourceState::Empty
    }

    fn mk_watcher<A: AsRef<Path>>(path: A, watch: bool) -> Option<PathWatcher> {
        if watch {
            PathWatcher::new(&path).ok()
        } else {
            None
        }
    }

    fn load<A: AsRef<Path>>(path: A, watch: bool) -> Self {
        let ret = std::fs::read_to_string(&path);
        let w = Self::mk_watcher(&path, watch);
        let p = path.as_ref().to_path_buf();
        match ret {
            Ok(s) => SourceState::Loaded((p, w, Source::new(s))),
            Err(e) => SourceState::Error((p, w, e)),
        }
    }

    pub fn load_updated_path<A: AsRef<Path>>(&mut self, path: A, watch: bool) -> FileStatus {
        if path.as_ref() == PathBuf::from("") {
            *self = SourceState::empty();
            return self.file_status();
        }
        *self = Self::load(path, watch);
        self.file_status()
    }

    pub fn reload_watched(&mut self, watch: bool) -> Option<FileStatus> {
        if watch {
            match self {
                SourceState::Loaded((p, Some(w), _)) | SourceState::Error((p, Some(w), _)) => {
                    if w.was_modified() {
                        *self = Self::load(p, watch);
                        return Some(self.file_status());
                    }
                }
                SourceState::Loaded((p, None, _)) | SourceState::Error((p, None, _)) => {
                    *self = Self::load(p, watch);
                    return Some(self.file_status());
                }
                SourceState::Empty => {}
            }
        } else {
            match self {
                SourceState::Loaded((_p, w, _)) | SourceState::Error((_p, w, _)) if w.is_some() => {
                    *w = None;
                }
                SourceState::Loaded(_) | SourceState::Error(_) | SourceState::Empty => {}
            }
        }
        None
    }

    fn file_status(&self) -> FileStatus {
        match self {
            SourceState::Loaded((path, _, source)) => {
                FileStatus::Loaded(path.display().to_string(), source.text.len(), source.time)
            }
            SourceState::Empty => FileStatus::NotLoaded,
            SourceState::Error((path, _, e)) => FileStatus::Error(format!(
                "Error loading file {}: {}",
                path.display(),
                e.to_string()
            )),
        }
    }

    pub fn get_source(&self) -> Option<&Source> {
        match self {
            SourceState::Loaded((_, _, s)) => Some(s),
            SourceState::Empty | SourceState::Error(_) => None,
        }
    }
}

pub struct PathWatcher {
    pub(crate) rx: crossbeam_channel::Receiver<Result<notify::Event, notify::Error>>,
    pub(crate) _watcher: RecommendedWatcher,
}

impl PathWatcher {
    pub fn new<A: AsRef<Path>>(path: A) -> Result<Self, notify::Error> {
        let path = path.as_ref().to_path_buf();
        let (tx, rx) = crossbeam_channel::unbounded();
        let mut watcher = RecommendedWatcher::new(tx, notify::Config::default())?;
        watcher.watch(&path, notify::RecursiveMode::NonRecursive)?;
        Ok(Self {
            rx,
            _watcher: watcher,
        })
    }
    pub fn was_modified(&self) -> bool {
        match self.rx.try_recv() {
            Ok(Ok(notify::Event {
                kind: notify::EventKind::Create(CreateKind::File),
                ..
            })) => true,
            Ok(Ok(notify::Event {
                kind: notify::EventKind::Modify(ModifyKind::Data(_)),
                ..
            })) => true,
            _ => false,
        }
    }
}
