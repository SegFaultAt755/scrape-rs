use crate::State;
use std::collections::VecDeque;
use std::num::NonZeroUsize;
use std::sync::{Arc, Condvar, Mutex};
use thiserror::Error;

pub struct ScrapeJob {
    url: String,
    status: JobStatus,
    index: Option<usize>,
}

impl ScrapeJob {
    pub fn new(url: String) -> Self {
        Self {
            url,
            status: JobStatus::Pending,
            index: None,
        }
    }

    pub fn new_indexed(url: String, index: usize) -> Self {
        Self {
            url,
            status: JobStatus::Pending,
            index: Some(index),
        }
    }

    pub fn url(&self) -> &str {
        &self.url
    }
    pub fn status(&self) -> &JobStatus {
        &self.status
    }
    pub fn index(&self) -> Option<usize> {
        self.index
    }
    pub fn set_status(&mut self, status: JobStatus) {
        self.status = status;
    }
}

#[derive(Default)]
pub struct Queue {
    jobs: Mutex<VecDeque<Option<ScrapeJob>>>,
    avaliable: Condvar,
}

impl Queue {
    pub fn new() -> Self {
        Self {
            jobs: Mutex::new(VecDeque::new()),
            avaliable: Condvar::new(),
        }
    }

    pub fn push(&self, job: ScrapeJob) {
        let mut jobs = self.jobs.lock().unwrap();
        jobs.push_back(Some(job));
        self.avaliable.notify_one();
    }

    pub fn next(&self) -> Option<ScrapeJob> {
        let mut jobs = self.jobs.lock().unwrap();
        loop {
            match jobs.pop_front() {
                Some(Some(job)) => return Some(job),
                Some(None) => return None,
                None => {}
            }
            jobs = self.avaliable.wait(jobs).unwrap();
        }
    }

    pub fn shutdown(&self, workers: usize) {
        let mut jobs = self.jobs.lock().unwrap();
        for _ in 0..workers {
            jobs.push_back(None);
        }
        self.avaliable.notify_all();
    }
}

pub fn worker(queue: Arc<Queue>, mut handler: impl FnMut(&mut ScrapeJob) + Send + 'static) {
    while let Some(mut job) = queue.next() {
        job.set_status(JobStatus::Running);
        handler(&mut job);
        job.set_status(JobStatus::Finished);
    }
}

pub enum JobStatus {
    Running,
    Pending,
    Finished,
    Failed,
    Waiting,
}

#[derive(Error, Debug)]
pub enum FetchError {
    #[error("Requested {requested} threads, but avaliable only {available}")]
    TooManyThreads {
        requested: NonZeroUsize,
        available: NonZeroUsize,
    },
    #[error("Could not determine the number of available threads: {0}")]
    ParallelismUnavailable(#[from] std::io::Error),
}

/// Handle to a background fetch: call `fetch_many` and keep working.
/// The result is collected via `wait()` (blocks until done) or `try_results()`.
pub struct FetchHandle {
    pub(crate) state: Arc<(Mutex<State>, Condvar)>,
    pub(crate) total: usize,
}

impl FetchHandle {
    /// true if all tasks have finished
    pub fn is_finished(&self) -> bool {
        let (lock, _) = &*self.state;
        lock.lock().unwrap().completed >= self.total
    }

    /// Returns results if everything is ready, otherwise None (does not block)
    pub fn try_results(&self) -> Option<Vec<Result<String, ureq::Error>>> {
        if !self.is_finished() {
            return None;
        }
        Some(self.collect())
    }

    /// How many tasks have finished already (does not block)
    pub fn completed(&self) -> usize {
        let (lock, _) = &*self.state;
        lock.lock().unwrap().completed
    }

    /// Takes and returns the already-completed results (as they arrive).
    /// Each completed result is returned exactly once — O(1) queue drain, not O(n) scan.
    pub fn ready_results(&self) -> Vec<(usize, Result<String, ureq::Error>)> {
        let (lock, _) = &*self.state;
        let mut state = lock.lock().unwrap();
        // O(1) swap of the ready queue; no scan over `results` (O(n)) any more
        let ready_indices = std::mem::take(&mut state.ready);
        let mut out = Vec::with_capacity(ready_indices.len());
        for index in ready_indices {
            // each ready index is guaranteed to have Some result
            if let Some(res) = state.results[index].take() {
                out.push((index, res));
            }
        }
        out
    }

    /// Blocks until all tasks finish and returns the results in original order
    pub fn wait(&self) -> Vec<Result<String, ureq::Error>> {
        let (lock, cvar) = &*self.state;
        let mut state = lock.lock().unwrap();
        while state.completed < self.total {
            state = cvar.wait(state).unwrap();
        }
        drop(state);
        self.collect()
    }

    fn collect(&self) -> Vec<Result<String, ureq::Error>> {
        let (lock, _) = &*self.state;
        let mut state = lock.lock().unwrap();
        // Clear ready since we drain everything in order
        state.ready.clear();
        let results = std::mem::take(&mut state.results);
        results.into_iter().map(|r| r.unwrap()).collect()
    }
}
