use std::thread;
use std::thread::{sleep, JoinHandle};
use std::time::Duration;


fn main() {
    let body = ureq::get("https://httpbin.io/json").call().unwrap().into_string().unwrap();

    println!("body: {}", body);

}
