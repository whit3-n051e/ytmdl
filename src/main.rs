
extern crate ytmdl;
extern crate tokio;

#[allow(dead_code)]
const TEST_URL: &str = "https://www.googleapis.com/youtube/v3/videos?id=7lCDEYXw3mM&key=AIzaSyDHTKjtUchUxUOzCtYW4V_h1zzcyd0P6c0&part=snippet,contentDetails,statistics,status";

#[tokio::main]
async fn main() {
    let resp = ytmdl::send_request(TEST_URL).await.expect("AAAAH");
    println!("{:#?}", resp)
}
