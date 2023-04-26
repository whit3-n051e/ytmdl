
#[allow(unused_imports)]
use serde_json::{Value, json};

extern crate ytmdl;
extern crate tokio;

// Sample vid: "BTYAsjAVa3I"

#[tokio::main]
async fn main() {
    let meta: ytmdl::Meta = ytmdl::download("BTYAsjAVa3I").await.unwrap();
    ytmdl::log(meta, "log_4.txt").unwrap();
}
