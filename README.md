# scrape-rs

A small, multithreaded web scraping library in Rust. It fetches web pages concurrently using a shared connection pool and provides simple CSS-selector-based HTML parsing.

## Features

- **Worker pool** — `init_worker_pool` spins up a configurable number of worker threads and returns a queue you can push URLs into at any time, plus a `FetchHandle` to read results
- **Incremental results** — process responses as soon as they arrive via `FetchHandle::ready_results`, or wait for everything with `wait()`
- **Single requests** — `fetch_link` for one-off GET/POST/PUT/PATCH/DELETE/HEAD/OPTIONS requests with optional body and content type
- **HTML parsing helpers** — `select_html`, `select_first`, and `select_all` for querying HTML with CSS selectors
- **Built-in timeouts** — 5 second default timeout, or bring your own `ureq::Agent`

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
scrape-rs = { path = "scrape-rs" }
```

### Fetch many URLs concurrently

```rust
use std::num::NonZeroUsize;
use scrape_rs::{init_worker_pool, parsers::{select_first, select_html}};

let urls: Vec<String> = (1..=10)
    .map(|i| format!("https://quotes.toscrape.com/page/{i}"))
    .collect();

let pool = init_worker_pool(NonZeroUsize::new(4).unwrap()).unwrap();
let handle = pool.handle();

for url in urls {
    pool.push(url);
}
pool.close();

loop {
    // Handle results as they complete
    for (_, res) in handle.ready_results() {
        if let Ok(html) = res {
            if let Some(title) = select_first(&html, "h1") {
                println!("{title}");
            }
        }
    }
    if handle.is_finished() {
        break;
    }
    std::thread::sleep(std::time::Duration::from_millis(10));
}
```

### Parse HTML with CSS selectors

```rust
use scrape_rs::parsers::{select_html, select_all, select_first};

for quote in select_html(html, ".quote") {
    let text = select_first(&quote, ".text").unwrap_or_default();
    let tags = select_all(&quote, ".tag");
    println!("{text} — tags: {tags:?}");
}
```

See the `test/` directory for a full working example.

## Building & Testing

```sh
cargo test --workspace
```

Tests spin up a local HTTP server on `127.0.0.1` — no network access required.

## License

This project is licensed under the [GNU Affero General Public License v3.0](LICENSE) (AGPL-3.0-or-later).
