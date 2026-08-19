use scraper::{Html, Selector};

pub fn select_all(html: &str, selector: &str) -> Vec<String> {
    let document = Html::parse_document(html);
    let Ok(selector) = Selector::parse(selector) else {
        return Vec::new();
    };
    document
        .select(&selector)
        .map(|el| el.text().collect::<Vec<_>>().join(" "))
        .collect()
}

pub fn select_first(html: &str, selector: &str) -> Option<String> {
    select_all(html, selector).into_iter().next()
}

pub fn select_html(html: &str, selector: &str) -> Vec<String> {
    let document = Html::parse_document(html);
    let Ok(selector) = Selector::parse(selector) else {
        return Vec::new();
    };
    document
        .select(&selector)
        .map(|el| el.html())
        .collect()
}