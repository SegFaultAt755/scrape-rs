#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

use scrape_rs::fetch_many;
use scrape_rs::parsers::{select_all, select_first, select_html};
use std::num::NonZeroUsize;

#[derive(Debug)]
#[allow(unused)]
struct Quote {
    text: String,
    author: String,
    tags: Vec<String>,
}

fn parse_quotes(html: &str) -> Vec<Quote> {
    select_html(html, ".quote")
        .into_iter()
        .map(|quote| Quote {
            text: select_first(&quote, ".text").unwrap_or_default(),
            author: select_first(&quote, ".author").unwrap_or_default(),
            tags: select_all(&quote, ".tag"),
        })
        .collect()
}

fn main() {
    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();

    let urls: Vec<String> = (1..=10)
        .map(|i| format!("https://quotes.toscrape.com/page/{}", i))
        .collect();

    let fetch_handle = fetch_many(urls, NonZeroUsize::new(1).unwrap()).unwrap();
    println!("threads started");

    // Keep polling for partial results and parse them as soon as they arrive
    loop {
        for (_, res) in fetch_handle.ready_results() {
            match res {
                Ok(html) => {
                    let quotes = parse_quotes(&html);
                    for quote in quotes {
                        println!("{:?}", quote);
                    }
                }
                Err(e) => eprintln!("fetch error: {e}"),
            }
        }

        if fetch_handle.is_finished() {
            break;
        }

        // Avoid wasting CPU — short sleep between polls
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    include!("tests.rs");
}
