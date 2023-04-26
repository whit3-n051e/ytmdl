extern crate ytmdl;
extern crate tokio;

#[allow(unused_imports)]
use ytmdl::debug;

// Sample vid: "BTYAsjAVa3I"
// https://www.youtube.com/watch?v=DK6IRG4CAbw

#[tokio::main]
async fn main() {
    debug("https://www.youtube.com/watch?v=DK6IRG4CAbw").await;
}
