extern crate ytmdl;
extern crate tokio;
extern crate hyper;

#[allow(unused_imports)]
use serde_json::json;
#[allow(unused_imports)]
use ytmdl::{
    get_meta
};


// https://www.youtube.com/watch?v=ZBh_mQl-2SQ

#[tokio::main]
async fn main() {
    println!("{:?}", "audio/webm".split('/').last())
}
