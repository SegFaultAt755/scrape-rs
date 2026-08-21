use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use uuid::Uuid;
use thiserror::Error;
use std::num::NonZeroUsize;

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
    loop {
        match queue.next() {
            Some(mut job) => {
                job.set_status(JobStatus::Running);
                handler(&mut job);
                job.set_status(JobStatus::Finished);
            }
            None => break,
        }
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