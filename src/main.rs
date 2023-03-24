
#[allow(unused_imports)]
use serde_json::{Value, json};

extern crate ytmdl;
extern crate tokio;

// Sample vid: "BTYAsjAVa3I"

#[tokio::main]
async fn main() {
    let meta = ytmdl::Meta::get("https://www.youtube.com/watch?v=DK6IRG4CAbw").await.unwrap();
    ytmdl::log(meta, "log_0.txt").unwrap();
}
