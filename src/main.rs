
#[allow(unused_imports)]
use serde_json::{Value, json};

extern crate ytmdl;
extern crate tokio;

#[tokio::main]
async fn main() {
    let vm = ytmdl::VideoMeta::from_vid("BTYAsjAVa3I").await.expect("AAAH");
    ytmdl::log(vm, "log.txt").expect("AAAAH");
}
