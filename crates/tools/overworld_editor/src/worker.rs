//! Builds levels on a background thread, so editing never waits for one. Jobs that pile up while
//! a build runs are skipped: only the newest document is built. With an export folder, the level
//! is also written there (pd-walk, watching it, reloads): after the editor has the build, and
//! only if no newer edit is waiting, so writing files never holds up what you see.

use overworld::textures::Library;
use overworld::{build, export, Doc, Level, Theme};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::Instant;

pub struct Job {
    pub doc: Doc,
    pub theme: Arc<Theme>,
    pub lib: Option<Arc<Library>>,
    pub export: Option<PathBuf>,
}

pub struct Done {
    pub doc: Doc,
    pub level: Result<Arc<Level>, String>,
    pub problems: Vec<String>,
    pub ms: f64,
}

pub enum Msg {
    Built(Done),
    /// A build was written out: its problems, now with the export's (missing textures).
    Exported(Vec<String>),
}

pub struct Worker {
    tx: Sender<Job>,
    rx: Receiver<Msg>,
}

impl Worker {
    pub fn new(wake: impl Fn() + Send + 'static) -> Worker {
        let (tx, jobs) = channel::<Job>();
        let (done, rx) = channel::<Msg>();
        std::thread::spawn(move || {
            let mut next: Option<Job> = None;
            loop {
                let mut job = match next.take() {
                    Some(j) => j,
                    None => match jobs.recv() {
                        Ok(j) => j,
                        Err(_) => break,
                    },
                };
                while let Ok(newer) = jobs.try_recv() {
                    job = newer;
                }
                let t = Instant::now();
                let level = build(&job.doc, &job.theme).map(Arc::new);
                let problems = level.as_ref().map(|l| l.problems.clone()).unwrap_or_default();
                let ms = t.elapsed().as_secs_f64() * 1000.0;
                let built = level.as_ref().ok().cloned();
                if done.send(Msg::Built(Done { doc: job.doc.clone(), level, problems, ms })).is_err() {
                    break;
                }
                wake();
                if let (Some(lvl), Some(out)) = (built, &job.export) {
                    // an edit arrived meanwhile: build that instead of writing this one out
                    if let Ok(newer) = jobs.try_recv() {
                        next = Some(newer);
                        continue;
                    }
                    let p = match export::write(&job.doc, &job.theme, &lvl, job.lib.as_deref(), out) {
                        Ok(p) => p,
                        Err(e) => {
                            let mut p = lvl.problems.clone();
                            p.push(format!("export: {e}"));
                            p
                        }
                    };
                    if done.send(Msg::Exported(p)).is_err() {
                        break;
                    }
                    wake();
                }
            }
        });
        Worker { tx, rx }
    }

    pub fn send(&self, job: Job) {
        let _ = self.tx.send(job);
    }

    /// What's arrived, in order.
    pub fn poll(&self) -> Vec<Msg> {
        self.rx.try_iter().collect()
    }
}
