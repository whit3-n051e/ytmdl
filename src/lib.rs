extern crate hyper;
extern crate hyper_tls;
extern crate tokio;
extern crate serde_json;
extern crate regex;

// VERY important constants
const API_KEY: &str = "AIzaSyA8eiZmM1FaDVjRy-df2KTyQ_vz_yYM39w";
const VID_REGEX: &str = r"^.*(?:(?:youtu\.be/|v/|vi/|u/w/|embed/)|(?:(?:watch)?\?v(?:i)?=|\&v(?:i)?=))([^#\&\?]*).*";

// Imports
use hyper::{
	Client, 
	Body,
	Request,
	Method,
	client::HttpConnector,
	body::to_bytes
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

// Important functions
pub async fn send_request(method: Method, url: &str, header: (&str, &str), body: &Value) -> Result<Value, Error> {
	let err: Error = Error::from(ErrorKind::InvalidData);
	let https: HttpsConnector<HttpConnector> = HttpsConnector::new();
	let client: Client<HttpsConnector<HttpConnector>> = Client::builder()
		.build::<_, Body>(https);

	match serde_json::to_string(body) {	
		Err(_) => Err(err),
		Ok(val) => match Request::builder()
			.method(method)
			.uri(url)
			.header(header.0, header.1)
			.body(Body::from(val)) {
				Err(_) => Err(err),
				Ok(val) => match client.request(val).await {
					Err(_) => Err(err),
					Ok(val) => match to_bytes(val.into_body()).await {
						Err(_) => Err(err),
						Ok(val) => match String::from_utf8(val.to_vec()) {
							Err(_) => Err(err),
							Ok(val) => match serde_json::from_str(&val) {
								Err(_) => Err(err),
								Ok(val) => Ok(val)
						}
					}
				}
			}
		}
	}
}
pub async fn get_video_data(vid: &str) -> Result<Value, Error> {
	let header: (&str, &str) = ("user-agent", "");
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
	let method: Method = Method::POST;
	let url: String = format!("https://www.youtube.com/youtubei/v1/player?key={}", API_KEY);
	let resp: Result<Value, Error> = send_request(method, &url, header, &body).await;
	match resp {
		Ok(val) => Ok(val),
		Err(err) => Err(err)
	}
}
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
pub fn to_vid(url: &str) -> Result<&str, Error> {
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

// One trait to rule them all
pub trait Grab {
	fn grab_b(&self, key: &str) -> bool;
	fn grab_s(&self, key: &str) -> String;
	fn grab_n(&self, key: &str) -> u64;
	fn grab_f(&self, key: &str) -> f64;
}
impl Grab for Value {
	fn grab_b(&self, key: &str) -> bool {
		let def_val: Value = json!(false);
		let v: &Value = self.get(key).unwrap_or(&def_val);
		v.as_bool().unwrap_or_default()
	}
	fn grab_s(&self, key: &str) -> String {
		let def_val: Value = json!("");
		let v: &Value = self.get(key).unwrap_or(&def_val);
		String::from(v.as_str().unwrap_or_default())
	}
	fn grab_n(&self, key: &str) -> u64 {
		let def_val: Value = json!(0);
		let v: &Value = self.get(key).unwrap_or(&def_val);
		v.as_u64().unwrap_or_default()
	}
	fn grab_f(&self, key: &str) -> f64 {
		let def_val: Value = json!(0.);
		let v: &Value = self.get(key).unwrap_or(&def_val);
		v.as_f64().unwrap_or_default()
	}
}

// Structs to get video metadata
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct AdaptiveAudioStream {
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

#[allow(dead_code)]
#[derive(Debug)]
pub struct VideoMeta {
	title: String,
	stream: AdaptiveAudioStream
}

impl AdaptiveAudioStream {
	pub fn from_sd(streaming_data: &Value) -> Option<Vec<Self>> {
		let af: &Vec<Value> = match streaming_data.get("adaptiveFormats") {
			None => return None,
			Some(val) => match val.as_array() {
				None => return None,
				Some(val) => val
			}
		};
		let mut aas_vec: Vec<Self> = vec![];

		for stream in af {
			let check_val: Option<&Value> = stream.get("audioQuality");
			if check_val.is_some() {
				aas_vec.push(Self {
					duration_ms: stream.grab_s("approxDurationMs").parse().unwrap_or_default(),
					audio_channels: stream.grab_n("audioChannels"),
					audio_sample_rate: stream.grab_s("audioSampleRate").parse().unwrap_or_default(),
					average_bitrate: stream.grab_n("averageBitrate"),
					bitrate: stream.grab_n("bitrate"),
					content_length: stream.grab_s("contentLength").parse().unwrap_or_default(),
					high_replication: stream.grab_b("highReplication"),
					itag: stream.grab_n("itag"),
					loudness_db: stream.grab_f("loudnessDb"),
					mime_type: stream.grab_s("mimeType"),
					url: stream.grab_s("url")
				});
			}
		};
		Some(aas_vec)
	}
}
impl VideoMeta {
	pub async fn get(url: &str) -> Result<Self, Error> {
		let vid: &str = match to_vid(url) {
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

		let streaming_data: &Value = match video_data.get("streamingData") {
			Some(val) => val,
			None => return Err(dataerror)
		};

		let streams: Vec<AdaptiveAudioStream> = match AdaptiveAudioStream::from_sd(streaming_data) {
			Some(val) => val,
			None => return Err(dataerror)
		};

		let mut max_quality_stream_index: usize = 0;

		for i in 1..streams.len() {
			if streams[i].bitrate > streams[max_quality_stream_index].bitrate {
				max_quality_stream_index = i;
			}
		};

		let best_stream: AdaptiveAudioStream = streams[max_quality_stream_index].clone();
		
		let video_meta: Self = Self {
			title: video_details.grab_s("title"),
			stream: best_stream
		};

		Ok(video_meta)
	}
}
