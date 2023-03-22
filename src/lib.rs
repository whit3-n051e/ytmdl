
extern crate hyper;
extern crate hyper_tls;
extern crate tokio;
extern crate serde_json;

use hyper::{
	Client, 
	Body,
	Request,
	Method,
	client::HttpConnector,
	Response,
	body::{
		to_bytes,
		Bytes
	}
};
use hyper_tls::HttpsConnector;
use std::{error::Error, io::Write, fmt::Debug};
use serde_json::{Value, json};
use std::fs::File;

const API_KEY: &str = "AIzaSyA8eiZmM1FaDVjRy-df2KTyQ_vz_yYM39w";

pub async fn send_request(method: Method, url: &str, header: (&str, &str), body: &Value) -> Result<Value, Box<dyn Error>> {
	let https: HttpsConnector<HttpConnector> = HttpsConnector::new();
	let client: Client<HttpsConnector<HttpConnector>> = Client::builder()
		.build::<_, Body>(https);
	let req_body: String = serde_json::to_string(body).expect("Error converting request header to string.");

	let req: Request<Body> = Request::builder()
		.method(method)
		.uri(url)
		.header(header.0, header.1)
		.body(Body::from(req_body))
		.expect("Error building request.");

	let res: Response<Body> = client.request(req).await.expect("Error receiving response.");
	let body_bytes: Bytes = to_bytes(res.into_body()).await.expect("Error converting response body to bytes.");
	let body_str: String = String::from_utf8(body_bytes.to_vec()).expect("Error converting response body to string.");
	let body_val: Value = serde_json::from_str(&body_str).expect("Error converting response body to json.");
	Ok(body_val)
}

pub async fn get_video_data(vid: &str) -> Result<Value, Box<dyn Error>> {
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
	let resp: Value = send_request(method, &url, header, &body).await?;
	Ok(resp)
}

pub fn write_file(content: &[u8], name: &str) -> Result<(), Box<dyn Error>> {
	let mut file: File = File::create(name)?;
	file.write_all(content)?;
	file.sync_all()?;
	Ok(())
}

pub fn log<T: Debug>(content: T, filename: &str) {
	let content_s: String = format!("{:#?}", content);
	let content_b: &[u8] = content_s.as_bytes();
	write_file(content_b, filename).expect("Failed to log.");
}

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

#[allow(dead_code)]
pub struct AdaptiveAudioStream {
	duration_ms: u64,
	audio_channels: u64,
	audio_quality: String,
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
pub struct VideoMeta {
	is_live: bool,
	is_private: bool,
	title: String,
	adaptive_audio_streams: Vec<AdaptiveAudioStream>
}



impl AdaptiveAudioStream {
	pub fn from_sd(streaming_data: &Value) -> Vec<Self> {
		let af: &Vec<Value> = streaming_data
			.get("adaptiveFormats")
			.expect("Could not find streams for the video.")
			.as_array()
			.expect("Could not find streams for the video.");
		let mut aas_vec: Vec<AdaptiveAudioStream> = vec![];

		for stream in af {
			let check_val: Option<&Value> = stream.get("audioQuality");
			if check_val != None {
				aas_vec.push(Self {
					duration_ms: stream.grab_s("approxDurationMs").parse().unwrap_or_default(),
					audio_channels: stream.grab_n("audioChannels"),
					audio_quality: stream.grab_s("audioQuality"),
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
		aas_vec
	}
}

impl VideoMeta {
	pub async fn from_vid(vid: &str) -> Result<Self, Box<dyn Error>> {
		let video_data: Value = get_video_data(vid).await?;
		let video_details: &Value = video_data.get("videoDetails").expect("");

		#[allow(unused_variables)]
		let streaming_data: &Value = video_data.get("streamingData").expect("");
		
		let video_meta: Self = Self {
			is_live: video_details.grab_b("isLiveContent"),
			is_private: video_details.grab_b("isPrivate"),
			title: video_details.grab_s("title"),
			adaptive_audio_streams: AdaptiveAudioStream::from_sd(streaming_data)
		};
		Ok(video_meta)
	}
}
