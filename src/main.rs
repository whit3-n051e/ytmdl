extern crate ytmdl;
extern crate tokio;
extern crate hyper;

#[allow(unused_imports)]
use serde_json::json;
#[allow(unused_imports)]
use ytmdl::{
    test,
    get_meta
};


// https://www.youtube.com/watch?v=ZBh_mQl-2SQ

#[tokio::main]
async fn main() {
    test("https://www.youtube.com/watch?v=ZBh_mQl-2SQ").await.expect("msg");
}
