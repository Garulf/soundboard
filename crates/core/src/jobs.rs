use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;

type Job = Box<dyn FnOnce() + Send>;

/// Small fixed thread pool for decoding and importing clips.
pub(crate) struct JobPool {
    tx: Sender<Job>,
}

impl JobPool {
    pub fn new() -> Self {
        let threads = thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(2)
            .clamp(1, 4);
        let (tx, rx) = channel::<Job>();
        let rx = Arc::new(Mutex::new(rx));
        for i in 0..threads {
            let rx = rx.clone();
            thread::Builder::new()
                .name(format!("clip-loader-{i}"))
                .spawn(move || worker(&rx))
                .expect("spawn clip loader thread");
        }
        Self { tx }
    }

    pub fn submit(&self, job: impl FnOnce() + Send + 'static) {
        let _ = self.tx.send(Box::new(job));
    }
}

fn worker(rx: &Mutex<Receiver<Job>>) {
    loop {
        let job = rx.lock().unwrap_or_else(PoisonError::into_inner).recv();
        match job {
            Ok(job) => job(),
            Err(_) => return,
        }
    }
}
