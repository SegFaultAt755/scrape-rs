pub mod parsers;
pub mod structs;

use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use structs::{worker, Queue, ScrapeJob};
use ureq::http::Method;

/// Handle to a background fetch: call `fetch_many` and keep working.
/// The result is collected via `wait()` (blocks until done) or `try_results()`.
pub struct FetchHandle {
    results: Arc<Mutex<Vec<Option<Result<String, ureq::Error>>>>>,
    done: Arc<(Mutex<usize>, Condvar)>,
    total: usize,
}

impl FetchHandle {
    /// true if all tasks have finished
    pub fn is_finished(&self) -> bool {
        *self.done.0.lock().unwrap() >= self.total
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
        *self.done.0.lock().unwrap()
    }

    /// Takes and returns the already-completed results (as they arrive).
    /// Each completed result is returned exactly once — a worker fills the slot,
    /// and this method takes it. Does not block.
    pub fn ready_results(&self) -> Vec<(usize, Result<String, ureq::Error>)> {
        let mut results = self.results.lock().unwrap();
        let mut out = Vec::new();
        for (index, slot) in results.iter_mut().enumerate() {
            if let Some(res) = slot.take() {
                out.push((index, res));
            }
        }
        out
    }

    /// Blocks until all tasks finish and returns the results in original order
    pub fn wait(&self) -> Vec<Result<String, ureq::Error>> {
        let (lock, cvar) = &*self.done;
        let mut done = lock.lock().unwrap();
        while *done < self.total {
            done = cvar.wait(done).unwrap();
        }
        self.collect()
    }

    fn collect(&self) -> Vec<Result<String, ureq::Error>> {
        let mut results = self.results.lock().unwrap();
        let results = std::mem::take(&mut *results);
        results.into_iter().map(|r| r.unwrap()).collect()
    }
}

/// Fetches links in the background on `num_threads` threads and returns immediately.
/// The result is collected through the handle.
pub fn fetch_many(urls: Vec<String>, num_threads: usize) -> FetchHandle {
    let queue = Arc::new(Queue::new());
    let total = urls.len();

    let results: Arc<Mutex<Vec<Option<Result<String, ureq::Error>>>>> =
        Arc::new(Mutex::new((0..total).map(|_| None).collect()));
    let done: Arc<(Mutex<usize>, Condvar)> = Arc::new((Mutex::new(0), Condvar::new()));

    for (index, url) in urls.into_iter().enumerate() {
        queue.push(ScrapeJob::new_indexed(url, index));
    }

let results_outer = Arc::clone(&results);
    let done_outer = Arc::clone(&done);
    thread::spawn(move || {
        let mut handles = Vec::new();
        for _ in 0..num_threads {
            let queue = Arc::clone(&queue);
            let results = Arc::clone(&results_outer);
            let done = Arc::clone(&done_outer);
            handles.push(thread::spawn(move || {
                worker(queue, move |job| {
                    let index = job.index().unwrap();
                    let res = fetch_link(job.url(), Method::GET, None, None);
                    let mut results = results.lock().unwrap();
                    results[index] = Some(res);
                    drop(results);
                    let (lock, cvar) = &*done;
                    let mut count = lock.lock().unwrap();
                    *count += 1;
                    cvar.notify_all();
                });
            }));
        }

        queue.shutdown(num_threads);

        for handle in handles {
            handle.join().unwrap();
        }
    });

    FetchHandle { results, done, total }
}

pub fn fetch_link(
    url: &str,
    method: Method,
    body: Option<String>,
    content_type: Option<&str>,
) -> Result<String, ureq::Error> {
    let response = match method {
        Method::GET => ureq::get(url).call()?,
        Method::DELETE => ureq::delete(url).call()?,
        Method::HEAD => ureq::head(url).call()?,
        Method::OPTIONS => ureq::options(url).call()?,
        Method::POST => send_body(ureq::post(url), body.as_deref(), content_type)?,
        Method::PUT => send_body(ureq::put(url), body.as_deref(), content_type)?,
        Method::PATCH => send_body(ureq::patch(url), body.as_deref(), content_type)?,
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


