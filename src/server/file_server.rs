use serde::Serialize;
use std::{
    convert::Infallible,
    error::Error,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
};
use tokio::fs::File;
use tokio_util::codec::{BytesCodec, FramedRead};
use warp::{
    http,
    hyper::{Body, Response},
    Filter, Rejection, Reply,
};
use warp::{
    http::StatusCode,
    reject::{custom, Reject},
};



#[derive(Debug, PartialEq)]
struct Retry;

impl Reject for Retry {}

/// start server
#[tokio::main]
pub async fn start(work_dir: PathBuf, socket_addr: SocketAddr) {
    let dist = warp::path("dist").and(warp::fs::dir(work_dir.join("dist")));
    let rustup = warp::path("rustup").and(warp::fs::dir(work_dir.join("rustup")));

    let crates_route = warp::path!("crates" / String / String / "download").and_then(
        move |name: String, version: String| {
            let path = work_dir.clone();
            async move { download_local_crates(path, &name, &version).await }
        },
    );

    // GET /dist/... => ./dist/..
    let routes = dist.or(rustup).or(crates_route).recover(handle_rejection);

    warp::serve(routes).run(socket_addr).await;
}

/// parse address with ip and port
pub fn parse_ipaddr(listen: Option<IpAddr>, port: Option<u16>) -> SocketAddr {
    let listen = listen.unwrap_or_else(|| IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
    let port = port.unwrap_or(8080);
    SocketAddr::new(listen, port)
}

async fn download_local_crates(
    work_dir: PathBuf,
    name: &str,
    version: &str,
) -> Result<Response<Body>, Rejection> {
    let full_path = work_dir
        .join("crates")
        .join(name)
        .join(format!("{}-{}.crate", name, version));

    let file = File::open(full_path).await.map_err(|_| custom(Retry))?;
    let meta = file.metadata().await.map_err(|_| custom(Retry))?;
    let stream = FramedRead::new(file, BytesCodec::new());

    let body = Body::wrap_stream(stream);

    let mut resp = Response::new(body);
    resp.headers_mut()
        .insert(http::header::CONTENT_LENGTH, meta.len().into());

    Ok(resp)
}

// fn download_from_remote() -> Result<Response<Body>, Rejection> {

// }

/// An API error serializable to JSON.
#[derive(Serialize)]
struct ErrorMessage {
    code: u16,
    message: String,
}

/// ### References Codes
///
/// - [warp](https://github.com/seanmonstar/warp)'s clone (example)[https://github.com/seanmonstar/warp/blob/master/examples/rejections.rs].
///
///
// This function receives a `Rejection` and tries to return a custom
// value, otherwise simply passes the rejection along.
async fn handle_rejection(err: Rejection) -> Result<impl Reply, Infallible> {
    let code;
    let message;

    println!("hanlde");
    if let Some(retry) = err.find::<Retry>() {
        println!("try to fetch for another source: {:?}", retry);
        //TODO retry
    }

    if err.is_not_found() {
        code = StatusCode::NOT_FOUND;
        message = "NOT_FOUND";
    } else if let Some(e) = err.find::<warp::filters::body::BodyDeserializeError>() {
        // This error happens if the body could not be deserialized correctly
        // We can use the cause to analyze the error and customize the error message
        message = match e.source() {
            Some(cause) => {
                if cause.to_string().contains("denom") {
                    "FIELD_ERROR: denom"
                } else {
                    "BAD_REQUEST"
                }
            }
            None => "BAD_REQUEST",
        };
        code = StatusCode::BAD_REQUEST;
    } else if let Some(_) = err.find::<warp::reject::MethodNotAllowed>() {
        // We can handle a specific error, here METHOD_NOT_ALLOWED,
        // and render it however we want
        code = StatusCode::METHOD_NOT_ALLOWED;
        message = "METHOD_NOT_ALLOWED";
    } else {
        // We should have expected this... Just log and say its a 500
        eprintln!("unhandled rejection: {:?}", err);
        code = StatusCode::INTERNAL_SERVER_ERROR;
        message = "UNHANDLED_REJECTION";
    }

    let json = warp::reply::json(&ErrorMessage {
        code: code.as_u16(),
        message: message.into(),
    });

    Ok(warp::reply::with_status(json, code))
}
