extern crate hyper;
extern crate hyper_tls;
extern crate tokio;
extern crate serde_json;
extern crate regex;

// Constants
const API_KEY: &str = "AIzaSyA8eiZmM1FaDVjRy-df2KTyQ_vz_yYM39w";
const VID_REGEX: &str = r"^.*(?:(?:youtu\.be/|v/|vi/|u/w/|embed/)|(?:(?:watch)?\?v(?:i)?=|\&v(?:i)?=))([^#\&\?]*).*";

// Imports
use hyper::{
	Client, 
	Body,
	Request,
	Method,
	client::HttpConnector,
	body::to_bytes,
	Response
};
use hyper_tls::HttpsConnector;
use std::{
	io::{
		Error, 
		ErrorKind,
		Write
	},
		fmt::Debug,
		fs::File
	};
use serde_json::{Value, json};
use regex::{
	Regex,
	Captures, 
	Match
};

// Enums, traits, structs
pub enum AudioContainer {
	M4A,
	WEBM
}

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

// Debug functions
pub fn log<T: Debug>(content: T, filename: &str) -> Result<(), Error> {
	let content_s: String = format!("{:#?}", content);
	let content_b: &[u8] = content_s.as_bytes();
	match write_file(content_b, filename) {
		Ok(_) => Ok(()),
		Err(err) => Err(err)
	}
}
pub fn read_line() -> Result<String, Error> {
	let mut s: String = String::new();
	match std::io::stdin().read_line(&mut s) {
		Ok(_) => Ok(s),
		Err(err) => Err(err)
	}
}

// System functions
pub fn write_file(content: &[u8], name: &str) -> Result<(), Error> {
	let mut file: File = match File::create(name) {
		Ok(val) => val,
		Err(err) => return Err(err)
	};
	match file.write_all(content) {
		Ok(_) => {},
		Err(err) => return Err(err)
	}
	match file.sync_all() {
		Ok(_) => Ok(()),
		Err(err) => Err(err)
	}
}
pub fn extract_vid(url: &str) -> Result<&str, Error> {
	let err: Error = Error::from(ErrorKind::InvalidInput);
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
pub fn best_stream(adaptive_streams: &[Value]) -> usize {
	let mut best_stream_id: usize = 0;
	let mut best_bitrate_yet: u64 = 0;
	for (id, strm) in adaptive_streams.iter().enumerate() {
		if strm.get("audioQuality").is_some() {
			let bitrate: u64 = strm.grab_n("bitrate");
			if bitrate > best_bitrate_yet {
				best_stream_id = id;
				best_bitrate_yet = bitrate;
			}
		}
	};
	best_stream_id
}

// Network functions
pub async fn request(data: RequestData) -> Result<Response<Body>, Error> {
	let error: Error = Error::from(ErrorKind::InvalidData);
	let https: HttpsConnector<HttpConnector> = HttpsConnector::new();
	let client: Client<HttpsConnector<HttpConnector>> = Client::builder()
		.build::<_, Body>(https);
	match serde_json::to_string(&data.body) {
		Err(_) => Err(error),
		Ok(val) => match Request::builder()
			.method(data.method)
			.uri(data.url)
			.header(data.header.key, data.header.value)
			.body(Body::from(val)) {
				Err(_) => Err(error),
				Ok(req) => match client.request(req).await {
					Err(_) => Err(error),
					Ok(resp) => Ok(resp)
				}
			}
	}
}
pub async fn get_video_data(vid: &str) -> Result<Value, Error> {
	let error: Error = Error::from(ErrorKind::InvalidData);
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
	let data: RequestData = RequestData { 
		method: Method::POST,
		url: format!("https://www.youtube.com/youtubei/v1/player?key={}", API_KEY),
		header: Header {
			key: String::from("user-agent"),
			value: String::new()
		},
		body
	};
	let resp: Result<Response<Body>, Error> = request(data).await;
	match resp {
		Err(_) => Err(error),
		Ok(val) => match to_bytes(val.into_body()).await {
			Err(_) => Err(error),
			Ok(val) => match String::from_utf8(val.to_vec()) {
				Err(_) => Err(error),
				Ok(val) => match serde_json::from_str(&val) {
					Err(_) => Err(error),
					Ok(val) => Ok(val)
				}
			}
		}
	}
}

// Implements
impl Meta {
	pub async fn get(url: &str) -> Result<Self, Error> {
		let vid: &str = match extract_vid(url) {
			Ok(val) => val,
			Err(err) => return Err(err)
		};
		let dataerror: Error = Error::from(ErrorKind::InvalidData);
		let video_data: Value = get_video_data(vid).await?;

		let video_details: &Value = match video_data.get("videoDetails") {
			Some(val) => val,
			None => return Err(dataerror)
		};

		if video_details.grab_b("isLiveContent") || video_details.grab_b("isPrivate") {
			return Err(dataerror);
		};

		let streams: Vec<Value> = match video_data.get("streamingData") {
			Some(val) => val.grab_a("adaptiveFormats"),
			None => return Err(dataerror)
		};

		let best_id: usize = best_stream(&streams);

		let video_meta: Self = Self {
			title: video_details.grab_s("title"),
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

		Ok(video_meta)
	}
}


// =======================================================================
// |                      UNDER DEVELOPMENT                              |
// =======================================================================


// Add downloading here
pub async fn download(url: &str) -> Result<Meta, Error> {
	let meta: Meta = match Meta::get(url).await {
		Ok(val) => val,
		Err(err) => return Err(err)
	};
	// let dl_url: &str = &meta.stream.url;

	Ok(meta)
}
