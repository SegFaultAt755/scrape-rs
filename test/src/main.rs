use std::num::NonZeroUsize;
use std::thread::sleep;
use scrape_rs::fetch_many;
use scrape_rs::parsers::{select_all, select_first, select_html};

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
    // println!("Multithread parsing\n\n\n\n");
    //
    // let mut threads = Vec::new();
    // for i in 1..11{
    //     let handle = spawn(move || {
    //         fetch_link(format!("https://quotes.toscrape.com/page/{}", i).as_str(), Method::GET, None, None)
    //
    //     });
    //     threads.push(handle);
    // }
    //
    //
    // let mut results = Vec::new();
    // for handle in threads {
    //     results.push(parse_quotes(handle.join().unwrap().unwrap().to_string().as_str()));
    // }
    //
    // for page in results {
    //     for quote in page {
    //         println!("{:?}", quote);
    //     }
    // }

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
