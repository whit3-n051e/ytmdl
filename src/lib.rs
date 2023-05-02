extern crate hyper;
extern crate hyper_tls;
extern crate tokio;
extern crate serde_json;
extern crate regex;
extern crate tempfile;
extern crate futures_core;
extern crate futures_util;
extern crate indicatif;

use hyper::{ Client, Body, Request, Method, HeaderMap, body::Bytes };
use hyper_tls::HttpsConnector;
use std::{ io::Write, fmt::Debug, fs::File, convert::From, path::PathBuf, cmp::min };
use serde_json::{ Value, json };
use regex::Regex;
use tempfile::Builder;
use futures_core::Stream;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};

// Constants
const API_KEY: &str = "AIzaSyA8eiZmM1FaDVjRy-df2KTyQ_vz_yYM39w";
const VID_REGEX: &str = r"^.*(?:(?:youtu\.be/|v/|vi/|u/w/|embed/)|(?:(?:watch)?\?v(?:i)?=|\&v(?:i)?=))([^#\&\?]*).*";

// Errors
#[derive(Debug)]
pub enum Error {
	IoError(std::io::Error),
	HyperError(hyper::Error),
	Utf8Error(std::string::FromUtf8Error),
	JsonError(serde_json::Error),
	HttpError(hyper::http::Error),
	RegexError(regex::Error),
	ToStrError(hyper::header::ToStrError),
	ParseIntError(std::num::ParseIntError),
	TemplateError(indicatif::style::TemplateError)
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
impl From<std::string::FromUtf8Error> for Error {
	fn from(err: std::string::FromUtf8Error) -> Self {
		Self::Utf8Error(err)
	}
}
impl From<serde_json::Error> for Error {
	fn from(err: serde_json::Error) -> Self {
		Self::JsonError(err)
	}
}
impl From<hyper::http::Error> for Error {
	fn from(err: hyper::http::Error) -> Self {
		Self::HttpError(err)
	}
}
impl From<regex::Error> for Error {
	fn from(err: regex::Error) -> Self {
		Self::RegexError(err)
	}
}
impl From<hyper::header::ToStrError> for Error {
	fn from(err: hyper::header::ToStrError) -> Self {
		Self::ToStrError(err)
	}
}
impl From<std::num::ParseIntError> for Error {
	fn from(err: std::num::ParseIntError) -> Self {
		Self::ParseIntError(err)
	}
}
impl From<indicatif::style::TemplateError> for Error {
	fn from(err: indicatif::style::TemplateError) -> Self {
		Self::TemplateError(err)
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

#[allow(dead_code)]
pub struct Response {
	head: HeaderMap,
	body: Body
}
impl Response {
	pub async fn receive(method: Method, url: String, body: Body) -> Result<Self, Error> {
		let response = Client::builder()
			.build::<_, Body>(HttpsConnector::new())
			.request(
				Request::builder()
					.method(method)
					.uri(url)
					.header("user-agent", "")
					.body(body)?
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
#[allow(dead_code)]
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
		let vdata: Value = Response::receive(
			Method::POST,
			format!("https://www.youtube.com/youtubei/v1/player?key={}", API_KEY),
			Body::from(serde_json::to_string(
				&json!({
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
				}))?
			)
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

		// See if the video download url signed, decipher if yes
		let url: String = match stream.get("url").is_some() {
			true => stream.grab("url"),
			false => decipher(stream.grab("signatureCipher"))?
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
				url
			}
		)
	}

	pub async fn download(self, to_tmp: bool) -> Result<PathBuf, Error> {

		// Decide if to download to temp folder or current folder
		let dest: PathBuf = match to_tmp {
			false => std::env::current_dir()?,
			true => Builder::new().prefix("ytmdl").tempdir()?.into_path()
		};

		// Get response from download url
		let response: Response = Response::receive(Method::GET, self.url, Body::empty()).await?;

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
		let pb: ProgressBar = ProgressBar::new(filesize);
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
}

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
pub fn decipher(cipher: String) -> Result<String, Error> {
	(cipher == String::new()).e()?;
	todo!();
}

// ------ UNDER DEVELOPMENT ------




/*
TO DO:

- Better error handling
- Get audio extensions
- Add deciphering

*/