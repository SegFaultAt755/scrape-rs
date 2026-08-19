pub mod parsers;

use ureq::http::Method;

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