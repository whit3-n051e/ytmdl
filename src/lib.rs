extern crate hyper;
extern crate hyper_tls;
extern crate tokio;
extern crate serde_json;
extern crate regex;
extern crate tempfile;
extern crate futures_core;
extern crate futures_util;
extern crate indicatif;

use hyper::{ Client, Request, Method, HeaderMap, body::Bytes, http::HeaderValue };
use hyper_tls::HttpsConnector;
use std::{ io::Write, fmt::Debug, fs::File, convert::From, path::PathBuf, cmp::min, str::FromStr };
use serde_json::{ Value, json };
use regex::Regex;
use tempfile::Builder;
use futures_core::Stream;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use serde::ser::Serialize;

// Constants
const GOOGLEAPI_URL: &str = "https://www.youtube.com/youtubei/v1/";
const API_KEY: &str = "AIzaSyA8eiZmM1FaDVjRy-df2KTyQ_vz_yYM39w";
const VID_REGEX: &str = r"^.*(?:(?:youtu\.be/|v/|vi/|u/w/|embed/)|(?:(?:watch)?\?v(?:i)?=|\&v(?:i)?=))([^#\&\?]*).*";

// Error messages
const DEV_EMSG: ErrorMessage = ErrorMessage {
	id: "",
	info: ""
};
const CIPHER_EMSG: ErrorMessage = ErrorMessage {
	id: "",
	info: ""
};

// Error message struct
#[derive(Debug)]
pub struct ErrorMessage {
	id: &'static str,
	info: &'static str
}
impl ErrorMessage {
	fn report(self) {
		println!("*** ERROR ***\nID: {}\n{}", self.id, self.info)
	}
}

// Errors
#[derive(Debug)]
pub enum Error {
	DevError,
	CipherError
}
impl<T: std::error::Error> From<T> for Error {
	fn from(_err: T) -> Self {
		Self::DevError
	}
}

pub trait Report {
	fn report(self);
}
impl Report for Result<(), Error> {
	fn report(self) {
		match self {
			Ok(()) => (),
			Err(err) => match err {
				Error::DevError => DEV_EMSG.report(),
				Error::CipherError => CIPHER_EMSG.report()
			}
		}
	}
}

// Making non-errors errors
pub trait Erroneous<T> {
	fn e(self) -> Result<T, Error>;
	fn r(self, err: Error) -> Result<T, Error>;
}
impl<T, E> Erroneous<T> for Result<T, E> {
	fn e(self) -> Result<T, Error> {
		match self {
			Ok(val) => Ok(val),
			Err(_) => Err(Error::DevError)
		}
	}
	fn r(self, err: Error) -> Result<T, Error> {
		match self {
			Ok(val) => Ok(val),
			Err(_) => Err(err)
		}
	}
}
impl<T> Erroneous<T> for Option<T> {
	fn e(self) -> Result<T, Error> {
		match self {
			Some(val) => Ok(val),
			None => Err(Error::DevError)
		}
	}
	fn r(self, err: Error) -> Result<T, Error> {
		match self {
			Some(val) => Ok(val),
			None => Err(err)
		}
	}
}
impl Erroneous<()> for bool {
	fn e(self) -> Result<(), Error> {
		match self {
			false => Ok(()),
			true => Err(Error::DevError)
		}
	}
	fn r(self, err: Error) -> Result<(), Error> {
		match self {
			false => Ok(()),
			true => Err(err)
		}
	}
}

// JSON
pub trait Grab<T> {
	fn grab(&self, key: &str) -> T;
}
impl Grab<bool> for Value {
	fn grab(&self, index: &str) -> bool {
		let v: Value = self.get(index).not_empty();
		v.as_bool().unwrap_or_default()
	}
}
impl Grab<String> for Value {
	fn grab(&self, index: &str) -> String {
		let v: Value = self.get(index).not_empty();
		String::from(v.as_str().unwrap_or_default())
	}
}
impl Grab<u64> for Value {
	fn grab(&self, index: &str) -> u64 {
		let v: Value = self.get(index).not_empty();
		v.as_u64().unwrap()
	}
}
impl Grab<f64> for Value {
	fn grab(&self, index: &str) -> f64 {
		let v: Value = self.get(index).not_empty();
		v.as_f64().unwrap_or_default()
	}
}
impl Grab<Vec<Value>> for Value {
	fn grab(&self, index: &str) -> Vec<Value> {
		let v: Value = self.get(index).not_empty();
		v.as_array().not_empty()
	}
}

pub trait NotEmpty<T> {
	fn not_empty(self) -> T;
}
impl NotEmpty<Value> for Option<&Value> {
	fn not_empty(self) -> Value {
		match self {
			Some(val) => val.to_owned(),
			None => json!({})
		}
	}
}
impl NotEmpty<Vec<Value>> for Option<&Vec<Value>> {
	fn not_empty(self) -> Vec<Value> {
		match self {
			Some(val) => val.to_owned(),
			None => Vec::new() as Vec<Value>
		}
	}
}
impl NotEmpty<HeaderValue> for Option<&HeaderValue> {
	fn not_empty(self) -> HeaderValue {
		match self {
			Some(val) => val.to_owned(),
			None => HeaderValue::from(0)
		}
	}
}

pub trait Parse<T> {
	fn parse(&self, key: &str) -> T;
}
impl<T: FromStr + Default> Parse<T> for Value {
	fn parse(&self, key: &str) -> T {
		(self as &dyn Grab<String>)
			.grab(key)
			.parse::<T>()
			.unwrap_or_default()
	}
}
impl<T: FromStr + Default> Parse<T> for HeaderMap {
	fn parse(&self, key: &str) -> T {
		self
			.get(key)
			.not_empty()
			.to_str()
			.unwrap_or_default()
			.parse::<T>()
			.unwrap_or_default()
	}
}

// DECIPHERING
pub trait Decipher {
	fn decipher(&self) -> Result<String, Error>;
}
impl Decipher for Value {
	fn decipher(&self) -> Result<String, Error> {
		let _cipher: String = match self.get("url") {
			Some(val) => return Ok(val.as_str().e()?.to_owned()),
			None => match self.get("signatureCipher") {
				Some(val) => val.as_str().e()?.to_owned(),
				None => return Err(Error::CipherError)
			}
		};
		todo!()
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
	pub async fn post(url: &str, body: Body) -> Result<Self, Error> {
		Self::receive(Method::POST, url, body).await
	}
	pub async fn get(url: &str) -> Result<Self, Error> {
		Self::receive(Method::GET, url, Body::default()).await
	}
	pub async fn receive(method: Method, url: &str, body: Body) -> Result<Self, Error> {
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
	pub url: String
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
		let url: String = format!("{}player?key={}", GOOGLEAPI_URL, API_KEY);
		let vdata: Value = Response::post(&url, Body::for_vid(vid))
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
		let mt: String = stream.grab("mimeType");
		let mt_regex: regex::Captures = Regex::new(r"(\w+)/(\w+);\scodecs=\W(\w+)\W")
			.unwrap()
			.captures(mt.as_str())
			.e()?;

		Ok(
			Self {
				title: video_details.grab("title"),
				duration_ms: stream.parse("approxDurationMs"),
				audio_channels: stream.grab("audioChannels"),
				audio_sample_rate: stream.parse("audioSampleRate"),
				average_bitrate: stream.grab("averageBitrate"),
				bitrate: stream.grab("bitrate"),
				content_length: stream.parse("contentLength"),
				high_replication: stream.grab("highReplication"),
				loudness_db: stream.grab("loudnessDb"),
				filetype: String::from(mt_regex.get(2).e()?.as_str()),
				codec: String::from(mt_regex.get(3).e()?.as_str()),
				url: stream.decipher()?
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
		let response: Response = Response::get(&self.url).await?;

		// Create the file to save the video
		let mut file: File = File::create(dest.join(self.title + "." + &self.filetype))?;

		// Get size of the video
		let filesize: u64 = response
			.head
			.parse("content-length");
		let mut bytestream = response.stream();

		// Set progress bar
		let mut downloaded: u64 = 0;
		pb.set_length(filesize);
		pb.set_style(
			ProgressStyle::default_bar()
				.template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")?
				.progress_chars("#>-")
		);
		pb.tick();
		pb.set_message("Downloading...");

		// Download/Update cycle
		while let Some(item) = bytestream.next().await {
			let chunk: Bytes = item?;
			file.write_all(&chunk)?;
			downloaded = min::<u64>(downloaded + (chunk.len() as u64), filesize);
			pb.set_position(downloaded);
		}

		// Finish
		pb.finish_with_message("Download complete.");
		Ok(dest)
	}
}


// ------ UNDER DEVELOPMENT ------






// ------------------------------
/* TO DO:
*
*  - Deciphering
*  - Converting to wav
*
*/
