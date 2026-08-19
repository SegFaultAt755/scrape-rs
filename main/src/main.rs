use scrape_rs::fetch_link;
use ureq::http::Method;


fn main() {
    let body = fetch_link(
        "https://httpbin.io/post",
        Method::POST,
        Some("custname=Fedor&custtel=123&size=large&topping=cheese&delivery=18%3A00&comments=hi".into()),
        Some("application/x-www-form-urlencoded"),
    );
    println!("{:?}", body);
}
