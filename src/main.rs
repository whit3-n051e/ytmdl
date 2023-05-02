extern crate ytmdl;
extern crate tokio;
extern crate hyper;

#[allow(unused_imports)]
use serde_json::json;
#[allow(unused_imports)]
use regex::Regex;


// https://www.youtube.com/watch?v=ZBh_mQl-2SQ

#[tokio::main]
async fn main() {
    //let meta: ytmdl::Meta = ytmdl::Meta::get("https://www.youtube.com/watch?v=ZBh_mQl-2SQ").await.unwrap();
    //ytmdl::download_file(meta.url, String::from("test"), std::env::current_dir().unwrap()).await.unwrap();
    //ytmdl::log(meta, "meta2.txt")

    /*
    let s: &str = "audio/webm; codecs=\"opus\"";
    let a = Regex::new(r"(\w+)/(\w+);\scodecs=\W(\w+)\W").unwrap().captures(s).expect("msg");
    println!("{:#?}", a);
    */
}
