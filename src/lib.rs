
extern crate hyper;
extern crate hyper_tls;
extern crate tokio;
extern crate serde_json;

#[allow(unused_imports)]
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
use std::error::Error;
use serde_json::Value;


pub async fn send_request(url: &str) -> Result<Value, Box<dyn Error>> {
    let https: HttpsConnector<HttpConnector> = HttpsConnector::new();
    let client: Client<HttpsConnector<HttpConnector>> = Client::builder()
        .build::<_, Body>(https);

    let req: Request<Body> = Request::builder()
        .method(Method::GET)
        .uri(url)
        .header("user-agent", "")
        .body(Body::from(""))?;

    let res: Response<Body> = client.request(req).await?;

    let body_bytes: Bytes = to_bytes(res.into_body()).await?;

    let body_str: String = String::from_utf8(body_bytes.to_vec())?;

    let body_value: Value = serde_json::from_str(&body_str)?;

    Ok(body_value)
}
