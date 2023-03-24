
#[allow(unused_imports)]
use serde_json::{Value, json};

extern crate ytmdl;
extern crate tokio;

// Sample vid: "BTYAsjAVa3I"

#[tokio::main]
async fn main() {
    let vd = ytmdl::Meta::get("BTYAsjAVa3I").await.unwrap();
    ytmdl::log(vd, "log_2.txt").unwrap();
}
