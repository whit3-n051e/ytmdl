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
    let meta: ytmdl::Meta = ytmdl::Meta::receive("https://www.youtube.com/watch?v=ZBh_mQl-2SQ").await.unwrap();
    ytmdl::download_file(meta.url, String::from("test"), std::env::current_dir().unwrap()).await.unwrap();
}
