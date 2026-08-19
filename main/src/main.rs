use scrape_rs::fetch_link;
use scrape_rs::parsers::{select_all, select_first, select_html};
use ureq::http::Method;

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
    let html = match fetch_link("https://quotes.toscrape.com", Method::GET, None, None) {
        Ok(body) => body,
        Err(e) => {
            eprintln!("failed to fetch: {e}");
            return;
        }
    };

    let quotes = parse_quotes(&html);
    for q in &quotes {
        println!("{:?}", q);
    }
}