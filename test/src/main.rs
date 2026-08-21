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

    // ── Timeout test: fetch from localhost:10000 ──
    // Server at localhost:10000 should respond with delay > timeout
    // (e.g., `sleep 2` in handler). Verify that ureq returns Error::Timeout.
    println!("\n--- Timeout test (localhost:10000) ---");
    {
        use std::time::Duration;
        use ureq::http::Method;
        use scrape_rs::{fetch_link_with_agent, fetch_many_with_agent};

        // 1) Single fetch_link_with_agent with short global timeout
        let fast_agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_millis(500)))
            .build()
            .new_agent();

        let url = "http://localhost:10000/";
        println!("fetch_link_with_agent GET {url} timeout=500ms ...");
        let start = std::time::Instant::now();
        match fetch_link_with_agent(&fast_agent, url, Method::GET, None, None) {
            Ok(body) => println!("[FAIL] expected timeout but got success: {} bytes in {:?}", body.len(), start.elapsed()),
            Err(ureq::Error::Timeout(t)) => println!("[OK] timeout as expected ({t:?}) in {:?}", start.elapsed()),
            Err(e) => println!("[INFO] other error (server down / no delay?): {e} in {:?}", start.elapsed()),
        }

        // 2) Same test via fetch_many_with_agent (background queue)
        let slow_urls = vec![
            "http://localhost:10000/".to_string(),
            "http://localhost:10000/delay".to_string(),
        ];
        println!("\nfetch_many_with_agent 2 urls timeout=500ms ...");
        let start = std::time::Instant::now();
        match fetch_many_with_agent(slow_urls, NonZeroUsize::new(2).unwrap(), fast_agent.clone()) {
            Ok(handle) => {
                // wait for all results (blocking)
                let results = handle.wait();
                for (i, res) in results.into_iter().enumerate() {
                    match res {
                        Ok(body) => println!("[FAIL] url#{i} expected timeout but got {} bytes", body.len()),
                        Err(ureq::Error::Timeout(t)) => println!("[OK] url#{i} timeout as expected ({t:?})"),
                        Err(e) => println!("[INFO] url#{i} other error: {e}"),
                    }
                }
                println!("fetch_many done in {:?}", start.elapsed());
            }
            Err(e) => eprintln!("fetch_many setup error: {e}"),
        }

        // 3) Control: same URL without short timeout should succeed (if server is alive)
        // now default = 5s (scrape_rs::DEFAULT_TIMEOUT), explicit 5s agent is identical to default
        let slow_agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(5)))
            .build()
            .new_agent();
        println!("\ncontrol fetch_with_agent (5s) GET {url} ...");
        let start = std::time::Instant::now();
        match fetch_link_with_agent(&slow_agent, url, Method::GET, None, None) {
            Ok(body) => println!("[OK] control success: {} bytes in {:?}", body.len(), start.elapsed()),
            Err(ureq::Error::Timeout(t)) => println!("[INFO] control also timed out ({t:?}) - server delays >5s? in {:?}", start.elapsed()),
            Err(e) => println!("[INFO] control error: {e} in {:?}", start.elapsed()),
        }

        // 4) Verify default timeout (5s) via scrape_rs::fetch_link / fetch_many without custom agent
        println!("\ndefault timeout check (scrape_rs::DEFAULT_TIMEOUT = {:?}) via fetch_link GET {url} ...", scrape_rs::DEFAULT_TIMEOUT);
        let start = std::time::Instant::now();
        match scrape_rs::fetch_link(url, Method::GET, None, None) {
            Ok(body) => println!("[OK] default fetch_link success: {} bytes in {:?}", body.len(), start.elapsed()),
            Err(ureq::Error::Timeout(t)) => println!("[OK] default fetch_link timed out as expected if server sleeps >5s ({t:?}) in {:?}", start.elapsed()),
            Err(e) => println!("[INFO] default fetch_link error: {e} in {:?}", start.elapsed()),
        }
        println!("\ndefault timeout check via fetch_many (default agent) ...");
        let start = std::time::Instant::now();
        match scrape_rs::fetch_many(vec![url.to_string()], NonZeroUsize::new(1).unwrap()) {
            Ok(handle) => {
                let results = handle.wait();
                for (i, res) in results.into_iter().enumerate() {
                    match res {
                        Ok(body) => println!("[OK] default fetch_many url#{i} success: {} bytes in {:?}", body.len(), start.elapsed()),
                        Err(ureq::Error::Timeout(t)) => println!("[OK] default fetch_many url#{i} timeout ({t:?}) in {:?}", start.elapsed()),
                        Err(e) => println!("[INFO] default fetch_many url#{i} error: {e} in {:?}", start.elapsed()),
                    }
                }
            }
            Err(e) => eprintln!("fetch_many setup error: {e}"),
        }
    }
}
