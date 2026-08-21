<div align="center">

![BANNER](./banner.svg)

### A small, multithreaded web scraping library in Rust. It fetches web pages concurrently using a shared connection pool and provides simple CSS-selector-based HTML parsing

</div>

---

<div align="center">

# Features

</div>

- **Concurrent fetching** — `fetch_many` fetches a list of URLs on a configurable number of worker threads
- **Incremental results** — process responses as soon as they arrive via `FetchHandle::ready_results`, or wait for everything with `wait()`
- **Single requests** — `fetch_link` for one-off GET/POST/PUT/PATCH/DELETE/HEAD/OPTIONS requests with optional body and content type
- **HTML parsing helpers** — `select_html`, `select_first`, and `select_all` for querying HTML with CSS selectors
- **Built-in timeouts** — 5 second default timeout, or bring your own `ureq::Agent`

---

<div align="center">

# Usage

</div>

Add to your `Cargo.toml`:

```toml
[dependencies]
scrape-rs = { path = "scrape-rs" }
```

<div align="center">

## Fetch many URLs concurrently

</div>

```rust
use std::num::NonZeroUsize;
use scrape_rs::{fetch_many, parsers::{select_first, select_html}};

let urls: Vec<String> = (1..=10)
    .map(|i| format!("https://quotes.toscrape.com/page/{i}"))
    .collect();

let handle = fetch_many(urls, NonZeroUsize::new(4).unwrap()).unwrap();

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

<div align="center">

## Parse HTML with CSS selectors

</div>

```rust
use scrape_rs::parsers::{select_html, select_all, select_first};

for quote in select_html(html, ".quote") {
    let text = select_first(&quote, ".text").unwrap_or_default();
    let tags = select_all(&quote, ".tag");
    println!("{text} — tags: {tags:?}");
}
```

See the `test/` directory for a full working example.

---

<div align="center">

# Building & Testing

</div>

```sh
cargo test --workspace
```

Tests spin up a local HTTP server on `127.0.0.1` — no network access required.

---

<div align="center">

# License

</div>

This project is licensed under the [GNU Affero General Public License v3.0](LICENSE) (AGPL-3.0-or-later).
