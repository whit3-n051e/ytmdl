extern crate hyper;
extern crate hyper_tls;
extern crate tokio;
extern crate serde_json;
extern crate regex;
extern crate tempfile;
extern crate futures_core;
extern crate futures_util;
extern crate indicatif;

use hyper::{ Client, Request, Method, HeaderMap, body::Bytes };
use hyper_tls::HttpsConnector;
use std::{ io::Write, fmt::Debug, fs::File, convert::From, path::PathBuf, cmp::min };
use serde_json::{ Value, json };
use regex::Regex;
use tempfile::Builder;
use futures_core::Stream;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use serde::ser::Serialize;

// Constants
const API_KEY: &str = "AIzaSyA8eiZmM1FaDVjRy-df2KTyQ_vz_yYM39w";
const VID_REGEX: &str = r"^.*(?:(?:youtu\.be/|v/|vi/|u/w/|embed/)|(?:(?:watch)?\?v(?:i)?=|\&v(?:i)?=))([^#\&\?]*).*";

// Errors
#[derive(Debug)]
pub enum Error {
	IoError(std::io::Error)
}
impl Default for Error {
	fn default() -> Self {
		Self::IoError(std::io::Error::from(std::io::ErrorKind::Other))
	}
}
impl<T: std::error::Error> From<T> for Error {
	fn from(_value: T) -> Self {
		Self::default()
	}
}

// Making non-errors errors
pub trait Erroneous<T> {
	fn e(self) -> Result<T, Error>;
}
impl<T> Erroneous<T> for Option<T> {
	fn e(self) -> Result<T, Error> {
		match self {
			Some(val) => Ok(val),
			None => Err(Error::default())
		}
	}
}
impl Erroneous<()> for bool {
	fn e(self) -> Result<(), Error> {
		match self {
			false => Ok(()),
			true => Err(Error::default())
		}
	}
}

// JSON
pub trait Grab<T> {
	fn grab(&self, key: &str) -> T;
}
impl Grab<bool> for Value {
	fn grab(&self, index: &str) -> bool {
		let default: Value = json!(false);
		let v: &Value = self.get(index).unwrap_or(&default);
		v.as_bool().unwrap_or_default()
	}
}
impl Grab<String> for Value {
	fn grab(&self, index: &str) -> String {
		let default: Value = json!("");
		let v: &Value = self.get(index).unwrap_or(&default);
		String::from(v.as_str().unwrap_or_default())
	}
}
impl Grab<u64> for Value {
	fn grab(&self, index: &str) -> u64 {
		let default: Value = json!(0);
		let v: &Value = self.get(index).unwrap_or(&default);
		v.as_u64().unwrap_or_default()
	}
}
impl Grab<f64> for Value {
	fn grab(&self, index: &str) -> f64 {
		let default: Value = json!(0.);
		let v: &Value = self.get(index).unwrap_or(&default);
		v.as_f64().unwrap_or_default()
	}
}
impl Grab<Vec<Value>> for Value {
	fn grab(&self, index: &str) -> Vec<Value> {
		let default: Value = json!([]);
		let v: &Value = self.get(index).unwrap_or(&default);
		v.as_array().unwrap().to_owned()
	}
}

// Make my own body because I can
pub struct Body(hyper::Body);
impl<T: Serialize> From<T> for Body {
	fn from(value: T) -> Self {
		Self(hyper::Body::from(serde_json::to_string(&json!(value)).unwrap()))
	}
}
impl Default for Body {
	fn default() -> Self {
		Self(hyper::Body::empty())
	}
}
impl Body {
	pub fn for_vid(vid: &str) -> Self {
		Self::from(
			json!({
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
			})
		)
	}
}

// Make my own response because using reqwest is unsportsmanlike
pub struct Response {
	head: HeaderMap,
	body: hyper::Body
}
impl Response {
	pub async fn receive(method: Method, url: String, body: Body) -> Result<Self, Error> {
		let response = Client::builder()
			.build::<_, hyper::Body>(HttpsConnector::new())
			.request(
				Request::builder()
					.method(method)
					.uri(url)
					.header("user-agent", "")
					.body(body.0)?
			)
				.await?;
		Ok(
			Self {
				head: response.headers().to_owned(),
				body: response.into_body()
			}
		)
	}

	pub fn stream(self) -> impl Stream<Item = Result<Bytes, hyper::Error>> {
		self.body
	}

	pub async fn to_json(self) -> Result<Value, Error> {
		Ok(serde_json::from_slice(&hyper::body::to_bytes(self.body).await?)?)
	}
}

// Structs
#[derive(Debug)]
pub struct Meta {
	pub title: String,
	pub duration_ms: u64,
	pub audio_channels: u64,
	pub audio_sample_rate: u64,
	pub average_bitrate: u64,
	pub bitrate: u64,
	pub content_length: u64,
	pub high_replication: bool,
	pub loudness_db: f64,
	pub filetype: String,
	pub codec: String,
	pub url: String,
	pub signature_cipher: String
}
impl Meta {
	pub async fn get(input: &str) -> Result<Self, Error> {

		// Get video id from url
		let vid: &str = match input.len() {
			11 => input,
			_ => Regex::new(VID_REGEX)?.captures(input).e()?.get(1).e()?.as_str()
		};
		(vid.len() != 11).e()?;

		// Receive raw video metadata
		let vdata: Value = Response::receive(
			Method::POST,
			format!("https://www.youtube.com/youtubei/v1/player?key={}", API_KEY),
			Body::for_vid(vid)
		)
			.await?
			.to_json()
			.await?;

		// Check if video is a livestream or private
		let video_details: &Value = vdata.get("videoDetails").e()?;
		(video_details as &dyn Grab<bool>).grab("isLiveContent").e()?;
		(video_details as &dyn Grab<bool>).grab("isPrivate").e()?;

		// Get best stream
		let stream: Value = {
			let mut sid: usize = 0;
			let mut best_bitrate_yet: u64 = 0;
			let streams: Vec<Value> = vdata
				.get("streamingData")
				.e()?
				.grab("adaptiveFormats");
			for (id, strm) in streams.iter().enumerate() {
				if strm.get("fps").is_none() && (strm as &dyn Grab<u64>).grab("audioChannels") >= 2 {
					let bitrate: u64 = strm.grab("bitrate");
					if bitrate > best_bitrate_yet {
						sid = id;
						best_bitrate_yet = bitrate;
					}
				}
			};
			streams[sid].clone()
		};

		// Get filetype and codec from mimeType
		let mt_regex: regex::Captures = Regex::new(r"(\w+)/(\w+);\scodecs=\W(\w+)\W")
			.unwrap()
			.captures(
				stream.get("mimeType")
				.e()?
				.as_str()
				.e()?
			).expect("msg");

		Ok(
			Self {
				title: video_details.grab("title"),
				duration_ms: (&stream as &dyn Grab<String>).grab("approxDurationMs").parse().unwrap_or_default(),
				audio_channels: stream.grab("audioChannels"),
				audio_sample_rate: (&stream as &dyn Grab<String>).grab("audioSampleRate").parse().unwrap_or_default(),
				average_bitrate: stream.grab("averageBitrate"),
				bitrate: stream.grab("bitrate"),
				content_length: (&stream as &dyn Grab<String>).grab("contentLength").parse().unwrap_or_default(),
				high_replication: stream.grab("highReplication"),
				loudness_db: stream.grab("loudnessDb"),
				filetype: String::from(mt_regex.get(2).e()?.as_str()),
				codec: String::from(mt_regex.get(3).e()?.as_str()),
				url: stream.grab("url"),
				signature_cipher: stream.grab("signatureCipher")
			}
		)
	}
	pub async fn download(self, to_tmp: bool, pb: ProgressBar) -> Result<PathBuf, Error> {

		// Decide if to download to temp folder or current folder
		let dest: PathBuf = match to_tmp {
			false => std::env::current_dir()?,
			true => Builder::new().prefix("ytmdl").tempdir()?.into_path()
		};

		// Get response from download url
		let response: Response = Response::receive(Method::GET, self.url, Body::default()).await?;

		// Create the file to save the video
		let mut file: File = File::create(dest.join(self.title + "." + &self.filetype))?;

		// Get size of the video
		let filesize: u64 = response
			.head
			.get("content-length")
			.e()?
			.to_str()?
			.parse()?;
		let mut stream = response.stream();

		// Set progress bar
		let mut downloaded: u64 = 0;
		pb.set_length(filesize);
		pb.set_style(
			ProgressStyle::default_bar()
				.template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")?
				.progress_chars("#>-")
		);
		pb.set_message("Downloading...");

		// Download/Update cycle
		while let Some(item) = stream.next().await {
			let chunk: Bytes = item?;
			file.write_all(&chunk)?;
			downloaded = min::<u64>(downloaded + (chunk.len() as u64), filesize);
			pb.set_position(downloaded);
		}

		// Finish
		pb.finish_with_message("Download complete.");
		Ok(dest)
	}
	pub fn decipher(self) -> Result<String, Error> {
		let _cipher = match self.url.as_str() {
			"" => match self.signature_cipher.as_str() {
				"" => return Err(Error::default()),
				_ => self.signature_cipher
			},
			_ => return Ok(self.url)
		};
		todo!()
	}
}


// ------ UNDER DEVELOPMENT ------






// ------------------------------
/* TO DO:
*
*  - Better error handling
*  - Add deciphering
*  - Add converting to wav
*
*/