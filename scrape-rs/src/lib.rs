pub mod parsers;
pub mod structs;

use std::collections::VecDeque;
use std::num::NonZeroUsize;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;
use structs::{Queue, ScrapeJob, worker};
use ureq::Agent;
use ureq::http::Method;
use crate::structs::FetchError;

/// Default timeout for all fetches via `fetch_many` / `fetch_link` (global).
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

/// Default agent: single `Agent` shared by all workers with `timeout_global = 5s`.
pub fn default_agent() -> Agent {
    Agent::config_builder()
        .timeout_global(Some(DEFAULT_TIMEOUT))
        .build()
        .new_agent()
}

struct State {
    results: Vec<Option<Result<String, ureq::Error>>>,
    completed: usize,
    ready: VecDeque<usize>,
}

/// Handle to a background fetch: call `fetch_many` and keep working.
/// The result is collected via `wait()` (blocks until done) or `try_results()`.
pub struct FetchHandle {
    state: Arc<(Mutex<State>, Condvar)>,
    total: usize,
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

/// Fetches links in the background on `num_threads` threads and returns immediately.
/// The result is collected through the handle.
/// All workers share a single `ureq::Agent` (connection pool + cookies).
pub fn fetch_many(urls: Vec<String>, num_threads: NonZeroUsize) -> Result<FetchHandle, FetchError> {
    let agent = default_agent();
    fetch_many_with_agent(urls, num_threads, agent)
}

/// Same as `fetch_many`, but uses a caller-provided `Agent`.
/// Allows custom TLS/proxy/config while still sharing one pool across all workers.
pub fn fetch_many_with_agent(
    urls: Vec<String>,
    num_threads: NonZeroUsize,
    agent: Agent,
) -> Result<FetchHandle, FetchError> {
    let available = std::thread::available_parallelism()
        .map_err(FetchError::ParallelismUnavailable)?;

    if num_threads.get() > available.get() {
        return Err(FetchError::TooManyThreads {
            requested: num_threads,
            available,
        });
    }

    let queue = Arc::new(Queue::new());
    let total = urls.len();

    let state: Arc<(Mutex<State>, Condvar)> = Arc::new((
        Mutex::new(State {
            results: (0..total).map(|_| None).collect(),
            completed: 0,
            ready: VecDeque::new(),
        }),
        Condvar::new(),
    ));

    for (index, url) in urls.into_iter().enumerate() {
        queue.push(ScrapeJob::new_indexed(url, index));
    }

    let state_outer = Arc::clone(&state);
    thread::spawn(move || {
        let mut handles = Vec::new();
        for _ in 0..num_threads.into() {
            let queue = Arc::clone(&queue);
            let state = Arc::clone(&state_outer);
            let agent = agent.clone();
            handles.push(thread::spawn(move || {
                worker(queue, move |job| {
                    let index = job.index().unwrap();
                    let res = fetch_link_with_agent(&agent, job.url(), Method::GET, None, None);
                    let (lock, cvar) = &*state;
                    let mut guard = lock.lock().unwrap();
                    guard.results[index] = Some(res);
                    guard.completed += 1;
                    guard.ready.push_back(index);
                    drop(guard);
                    cvar.notify_all();
                });
            }));
        }

        queue.shutdown(usize::from(num_threads));

        for handle in handles {
            handle.join().unwrap();
        }
    });

    Ok(FetchHandle { state, total })
}

/// Fetch a single URL using the shared-agent pattern.
/// This is a convenience wrapper that creates a one-off agent. Prefer
/// `fetch_link_with_agent` when you already have an `Agent`.
pub fn fetch_link(
    url: &str,
    method: Method,
    body: Option<String>,
    content_type: Option<&str>,
) -> Result<String, ureq::Error> {
    let agent = default_agent();
    fetch_link_with_agent(&agent, url, method, body, content_type)
}

/// Fetch a single URL using an explicit `Agent` (shared pool).
pub fn fetch_link_with_agent(
    agent: &Agent,
    url: &str,
    method: Method,
    body: Option<String>,
    content_type: Option<&str>,
) -> Result<String, ureq::Error> {
    let response = match method {
        Method::GET => agent.get(url).call()?,
        Method::DELETE => agent.delete(url).call()?,
        Method::HEAD => agent.head(url).call()?,
        Method::OPTIONS => agent.options(url).call()?,
        Method::POST => send_body(agent.post(url), body.as_deref(), content_type)?,
        Method::PUT => send_body(agent.put(url), body.as_deref(), content_type)?,
        Method::PATCH => send_body(agent.patch(url), body.as_deref(), content_type)?,
        _ => return Err(ureq::Error::StatusCode(405)),
    };
    Ok(response.into_body().read_to_string()?)
}

fn send_body(
    builder: ureq::RequestBuilder<ureq::typestate::WithBody>,
    body: Option<&str>,
    content_type: Option<&str>,
) -> Result<ureq::http::Response<ureq::Body>, ureq::Error> {
    let builder = match content_type {
        Some(ct) => builder.header("Content-Type", ct),
        None => builder,
    };
    match body {
        Some(b) => builder.send(b),
        None => builder.send_empty(),
    }
}
