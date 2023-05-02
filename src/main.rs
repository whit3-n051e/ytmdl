extern crate ytmdl;
extern crate tokio;
extern crate hyper;

use std::fmt::Debug;
use std::fs::File;
use std::io::Write;

pub fn log<T: Debug>(content: T, filename: &str) {
	let str: String = format!("{:#?}", content);
	let content: &[u8] = str.as_bytes();
	let mut file: File = match File::create(filename) {
		Ok(val) => val,
		Err(_) => {
			println!("LOG: Could not create file.");
			return
		}
	};
	match file.write_all(content) {
		Ok(_) => {},
		Err(_) => {
			println!("LOG: Could not write to file.");
			return
		}
	}
	match file.sync_all() {
		Ok(_) => println!("LOG: Success: {filename}"),
		Err(_) => println!("LOG: Could not sync.")
	};
}


// https://www.youtube.com/watch?v=ZBh_mQl-2SQ

#[tokio::main]
async fn main() {
    //let meta: ytmdl::Meta = ytmdl::Meta::get("https://www.youtube.com/watch?v=ZBh_mQl-2SQ").await.unwrap();
    //ytmdl::download_file(meta.url, String::from("test"), std::env::current_dir().unwrap()).await.unwrap();
    //ytmdl::log(meta, "meta2.txt")
    let pb = indicatif::ProgressBar::new(0);
    pb.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.red/blue}])").unwrap()
            .progress_chars("->-")
    );
    pb.set_message("msg");
    println!("e");
    /*
    let s: &str = "audio/webm; codecs=\"opus\"";
    let a = Regex::new(r"(\w+)/(\w+);\scodecs=\W(\w+)\W").unwrap().captures(s).expect("msg");
    println!("{:#?}", a);
    */
}
