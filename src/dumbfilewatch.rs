use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use std::sync::mpsc::channel;
use std::sync::mpsc;
use std::thread;
use thread::JoinHandle;


#[derive(Debug, Default)]
pub struct DFWatcher {
    pub path: PathBuf,

    // used to check for updates:
    last_len: u64,
    last_modified: Option<SystemTime>,
    last_created: Option<SystemTime>,
    last_accessed: Option<SystemTime>,
}

pub struct DFOwner {
    t: JoinHandle<()>,
    rx: mpsc::Receiver<CheckResponse>,
}

#[derive(Debug)]
pub enum CheckResponse {
    HasUpdated,
    IsSame,
    Err
}

impl DFWatcher {
    pub fn new<P: AsRef<Path>>(file: P) -> DFWatcher {
        let mut watcher = DFWatcher::default();
        watcher.path = file.as_ref().into();
        watcher.check();
        watcher
    }

    pub fn check(&mut self) -> CheckResponse {
        let metadata = match self.path.metadata() {
            Ok(m) => m,
            Err(_) => {
                return CheckResponse::Err;
            }
        };
        let len = metadata.len();
        let this_modified = metadata.modified().ok();
        let this_created = metadata.created().ok();
        let this_accessed = metadata.accessed().ok();
        let same_len = len == self.last_len;
        let same_mod = this_modified == self.last_modified;
        let same_create = this_created == self.last_created;
        let same_accessed = this_accessed == self.last_accessed;
        if same_len && same_mod && same_create && same_accessed {
            CheckResponse::IsSame
        } else {
            self.last_len = len;
            self.last_created = this_created;
            self.last_accessed = this_accessed;
            self.last_modified = this_modified;
            CheckResponse::HasUpdated
        }
    }
}

impl DFOwner {
    pub fn start(millis: u64, watch: Vec<DFWatcher>) -> DFOwner {
        let (tx, rx) = channel();
        let t = thread::spawn(move || {
            let mut watch = watch;
            let sleep_dur = Duration::from_millis(millis);
            loop {
                std::thread::sleep(sleep_dur);
                for w in &mut watch {
                    let response = w.check();
                    tx.send(response).unwrap();
                }
            }
        });
        DFOwner {
            rx, t,
        }
    }

    pub fn get_events(&mut self) -> Vec<CheckResponse> {
        let mut events = vec![];
        let mut should_try = true;
        while should_try {
            match self.rx.try_recv() {
                Ok(e) => {
                    events.push(e);
                }
                Err(_) => {
                    should_try = false;
                }
            }
        }
        events
    }

    pub fn should_update(&mut self) -> bool {
        let events = self.get_events();
        let mut should_update = false;
        for ev in events {
            match ev {
                CheckResponse::HasUpdated => {
                    should_update = true;
                    break;
                }
                _ => {}
            }
        }
        should_update
    }
}
