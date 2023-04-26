extern crate hyper;
extern crate hyper_tls;
extern crate tokio;
extern crate serde_json;
extern crate regex;
extern crate reqwest;
extern crate tempfile;

// Constants
const API_KEY: &str = "AIzaSyA8eiZmM1FaDVjRy-df2KTyQ_vz_yYM39w";
const VID_REGEX: &str = r"^.*(?:(?:youtu\.be/|v/|vi/|u/w/|embed/)|(?:(?:watch)?\?v(?:i)?=|\&v(?:i)?=))([^#\&\?]*).*";
const ERRORS: [&str; 7] = [
	/* 0 */ "Incorrect video URL/ID.",
	/* 1 */ "Could not convert response to JSON",
	/* 2 */ "Could not get any metadata for this video.",
	/* 3 */ "This video is a livestream; I am not downloading that.",
	/* 4 */ "This video is private.",
	/* 5 */ "Could not get any streams from this video.",
	/* 6 */ "Could not get response from server, probably due to a connection error."
];

// Imports
use hyper::{ Client, Body, Request, Method, client::HttpConnector, body::to_bytes, Response };
use hyper_tls::HttpsConnector;
use std::{ io::{ Write, copy }, fmt::Debug, fs::File, convert::From };
use serde_json::{ Value, json};
use regex::{ Regex, Captures, Match };
use tempfile::{ Builder, TempDir };

// Structs
#[allow(dead_code)]
#[derive(Debug)]
pub struct Meta {
	title: String,
	duration_ms: u64,
	audio_channels: u64,
	audio_sample_rate: u64,
	average_bitrate: u64,
	bitrate: u64,
	content_length: u64,
	high_replication: bool,
	itag: u64,
	loudness_db: f64,
	mime_type: String,
	url: String
}
pub struct Header {
	key: String,
	value: String
}
pub struct RequestData {
	method: Method,
	url: String,
	header: Header,
	body: Value
}

// Implementations
pub trait Grab {
	fn grab_b(&self, key: &str) -> bool;
	fn grab_s(&self, key: &str) -> String;
	fn grab_n(&self, key: &str) -> u64;
	fn grab_f(&self, key: &str) -> f64;
	fn grab_a(&self, key: &str) -> Vec<Value>;
}
impl Grab for Value {
	fn grab_b(&self, index: &str) -> bool {
		let default: Value = json!(false);
		let v: &Value = self.get(index).unwrap_or(&default);
		v.as_bool().unwrap_or_default()
	}
	fn grab_s(&self, index: &str) -> String {
		let default: Value = json!("");
		let v: &Value = self.get(index).unwrap_or(&default);
		String::from(v.as_str().unwrap_or_default())
	}
	fn grab_n(&self, index: &str) -> u64 {
		let default: Value = json!(0);
		let v: &Value = self.get(index).unwrap_or(&default);
		v.as_u64().unwrap_or_default()
	}
	fn grab_f(&self, index: &str) -> f64 {
		let default: Value = json!(0.);
		let v: &Value = self.get(index).unwrap_or(&default);
		v.as_f64().unwrap_or_default()
	}
	fn grab_a(&self, index: &str) -> Vec<Value> {
		let default: Value = json!([]);
		let v: &Value = self.get(index).unwrap_or(&default);
		v.as_array().unwrap().to_owned()
	}
}
impl Default for Header {
	fn default() -> Self {
		Self {
			key: String::from("user-agent"),
			value: String::new()
		}
	}
}
impl Meta {
	pub async fn get(url: &str) -> Result<Self, Error> {
		let (title, streams) = get_raw_meta(url).await?;
		let best_id: usize = best_stream(&streams)?;

		let meta: Self = Self {
			title,
			duration_ms: streams[best_id].grab_s("approxDurationMs").parse().unwrap_or_default(),
			audio_channels: streams[best_id].grab_n("audioChannels"),
			audio_sample_rate: streams[best_id].grab_s("audioSampleRate").parse().unwrap_or_default(),
			average_bitrate: streams[best_id].grab_n("averageBitrate"),
			bitrate: streams[best_id].grab_n("bitrate"),
			content_length: streams[best_id].grab_s("contentLength").parse().unwrap_or_default(),
			high_replication: streams[best_id].grab_b("highReplication"),
			itag: streams[best_id].grab_n("itag"),
			loudness_db: streams[best_id].grab_f("loudnessDb"),
			mime_type: streams[best_id].grab_s("mimeType"),
			url: streams[best_id].grab_s("url")
		};

		Ok(meta)
	}
}

// One Error For Them All
#[derive(Debug)]
pub enum Error {
	IoError(std::io::Error),
	HyperError(hyper::Error),
	ReqwestError(reqwest::Error),
	FromUTF8Error(std::string::FromUtf8Error),
	JSONError(serde_json::Error),
	HTTPError(hyper::http::Error)
}
impl Default for Error {
	fn default() -> Self {
		Self::IoError(std::io::Error::from(std::io::ErrorKind::Other))
	}
}
impl From<std::io::Error> for Error {
	fn from(err: std::io::Error) -> Self {
		Self::IoError(err)
	}
}
impl From<hyper::Error> for Error {
	fn from(err: hyper::Error) -> Self {
		Self::HyperError(err)
	}
}
impl From<reqwest::Error> for Error {
	fn from(err: reqwest::Error) -> Self {
		Self::ReqwestError(err)
	}
}
impl From<std::string::FromUtf8Error> for Error {
	fn from(err: std::string::FromUtf8Error) -> Self {
		Self::FromUTF8Error(err)
	}
}
impl From<serde_json::Error> for Error {
	fn from(err: serde_json::Error) -> Self {
		Self::JSONError(err)
	}
}
impl From<hyper::http::Error> for Error {
	fn from(err: hyper::http::Error) -> Self {
		Self::HTTPError(err)
	}
}

pub trait Erroneous<T> {
	fn e(self, n: usize) -> Result<T, Error>;
}
impl<T, E> Erroneous<T> for Result<T, E> {
	fn e(self, n: usize) -> Result<T, Error> {
		match self {
			Ok(val) => Ok(val),
			Err(_) => {
				println!("ERROR: {}", ERRORS[n]);
				Err(Error::default())
			}
		}
	}
}
impl<T> Erroneous<T> for Option<T> {
	fn e(self, n: usize) -> Result<T, Error> {
		match self {
			Some(val) => Ok(val),
			None => {
				println!("ERROR: {}", ERRORS[n]);
				Err(Error::default())
			}
		}
	}
}
impl Erroneous<()> for bool {
	fn e(self, n: usize) -> Result<(), Error> {
		match self {
			false => Ok(()),
			true => {
				println!("ERROR: {}", ERRORS[n]);
				Err(Error::default())
			}
		}
	}
}

// Debug functions
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
pub async fn debug(input: &str) {
	let vid: &str = match vid_from_url(input) {
		Ok(val) => val,
		Err(_) => {println!("Incorrect input."); return}
	};
	let vdata: (String, Vec<Value>) = match get_raw_meta(vid).await {
		Ok(val) => val,
		Err(_) => {println!("Could not get video data."); return}
	};
	log(vdata, "vdata.txt");
	/*
	let meta: Meta = match Meta::get(vid).await {
		Ok(val) => val,
		Err(_) => {println!("Could not get meta"); return}
	};
	log(meta, "meta.txt");
	*/
}

// Calc functions
pub fn vid_from_url(url: &str) -> Result<&str, Error> {
	let err: Error = Error::default();
	if url.len() == 11 {
		return Ok(url);
	}
	let vid_regex: Regex = Regex::new(VID_REGEX).unwrap();
	let vid_cap: Captures = match vid_regex.captures(url) {
		None => return Err(err),
		Some(val) => val
	};
	let vid_match: Match = match vid_cap.get(1) {
		None => return Err(err),
		Some(val) => val
	};
	let vid: &str = vid_match.as_str();
	match vid.len() {
		11 => Ok(vid),
		_ => Err(err)
	}
}
pub fn best_stream(adaptive_streams: &[Value]) -> Result<usize, Error> {
	let mut best_stream_id: usize = 0;
	let mut best_bitrate_yet: u64 = 0;
	for (id, strm) in adaptive_streams.iter().enumerate() {
		if strm.get("audioQuality").is_some() {
			let bitrate: u64 = strm.grab_n("bitrate");
			let url: String = strm.grab_s("url");
			let audio_channels = strm.grab_n("audioChannels");
			if (bitrate > best_bitrate_yet) && (url != String::new()) && (audio_channels != 0) {
				best_stream_id = id;
				best_bitrate_yet = bitrate;
			}
		}
	};
	match adaptive_streams[best_stream_id].grab_s("url").is_empty() {
		false => Ok(best_stream_id),
		true => Err(Error::default())
	}
}

// Network functions
pub async fn request(data: RequestData) -> Result<Response<Body>, Error> {
	let https: HttpsConnector<HttpConnector> = HttpsConnector::new();
	let client: Client<HttpsConnector<HttpConnector>> = Client::builder()
		.build::<_, Body>(https);
	client.request(Request::builder()
		.method(data.method)
		.uri(data.url)
		.header(data.header.key, data.header.value)
		.body(Body::from(serde_json::to_string(&data.body)?))?).await.e(6) // Error 6
}
pub async fn get_raw_meta(url: &str) -> Result<(String, Vec<Value>), Error> {
	let vid: &str = vid_from_url(url).e(0)?;  // Error 0
	let method: Method = Method::POST;
	let url: String = format!("https://www.youtube.com/youtubei/v1/player?key={}", API_KEY);
	let header: Header = Header::default();
	let body: Value = json!({
        "videoId": vid,
        "context": {
            "client": {
                "clientName": "TVHTML5_SIMPLY_EMBEDDED_PLAYER",
				"clientVersion": "2.0"
            },
			"thirdParty": {
				"embedUrl": "https://www.youtube.com"
			}
        }
    });
	let vdata: Value = serde_json::from_str::<Value>(&String::from_utf8(to_bytes(request(RequestData{method,url,header,body}).await?.into_body()).await?.to_vec())?).e(1)?; // Error 1
	let video_details: &Value = vdata.get("videoDetails").e(2)?;  // Error 2
	video_details.grab_b("isLiveContent").e(3)?;  // Error 3
	video_details.grab_b("isPrivate").e(4)?;  // Error 4
	let streaming_data = vdata.get("streamingData").e(5)?;  // Error 5
	Ok((video_details.grab_s("title"), streaming_data.grab_a("adaptiveFormats")))
}
pub async fn download(input: &str) -> Result<Meta, Error> {
	let meta: Meta = Meta::get(input).await?;
	let tmp_dir: TempDir = Builder::new().prefix("example").tempdir()?;
	let response: reqwest::Response = reqwest::get(&meta.url).await?;

	let mut dest: File = {
		let fname: &str = response
			.url()
			.path_segments()
			.and_then(|segments: std::str::Split<char>| segments.last())
			.and_then(|name: &str| if name.is_empty() { None } else { Some(name) })
			.unwrap_or("tmp.bin");
		let fname: std::path::PathBuf = tmp_dir.path().join(fname);
		File::create(fname)?
	};
	let content = response.text().await?;
	copy(&mut content.as_bytes(), &mut dest)?;
	Ok(meta)
}
