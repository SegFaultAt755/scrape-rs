use scrape_rs::init_worker_pool;
use scrape_rs::parsers::{select_all, select_first, select_html};
use std::num::NonZeroUsize;
use std::thread::sleep;

#[derive(Debug)]
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
    let urls: Vec<String> = (1..=100)
        .map(|i| format!("https://quotes.toscrape.com/page/{}", i))
        .collect();

    let pool = init_worker_pool(NonZeroUsize::new(1).unwrap()).unwrap();
    let fetch_handle = pool.handle();
    println!("threads started");

    for url in urls {
        pool.push(url);

    }
    pool.close(); // Signal that workers can finish, and that there will not be any new task.

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
