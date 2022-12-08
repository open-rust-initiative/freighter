//!
//!
//!
//!
//!
//!

use std::{
    borrow::BorrowMut,
    convert::Infallible,
    error::Error,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
};

use log::{error, info};
use serde::Serialize;
use tokio::{fs::File, io::AsyncWriteExt};
use tokio_util::codec::{BytesCodec, FramedRead};
use warp::{
    http,
    http::StatusCode,
    hyper::{Body, Response, Uri},
    reject,
    reject::Reject,
    Filter, Rejection, Reply,
};

use crate::{
    config::Config,
    errors::{FreightResult, FreighterError},
};

use super::git_protocal::{GitCommand, GitProtocal};

#[derive(Debug, PartialEq, Clone)]
struct MissingFile {
    pub uri: Uri,
}
impl Reject for MissingFile {}

#[derive(Debug)]
pub struct FileServer {
    pub cert_path: Option<PathBuf>,
    pub key_path: Option<PathBuf>,
    pub addr: Option<IpAddr>,
    pub port: Option<u16>,
}

/// start server
#[tokio::main]
pub async fn start(config: &Config, file_server: &FileServer) {
    tracing_subscriber::fmt::init();

    let work_dir = config.work_dir.clone().unwrap();
    let serve_index = config.crates.serve_index.clone();

    let rustup_backup_domain = config.rustup.backup_domain.clone().unwrap_or_else(|| {
        vec![
            String::from("localhost"),
            String::from("https://static.rust-lang.org"),
            String::from("https://rsproxy.cn"),
        ]
    });

    let work_dir2 = work_dir.clone();
    let rustup_backup_domain2 = rustup_backup_domain.clone();
    let dist = warp::path("dist")
        .and(warp::path::tail())
        .and_then(move |tail: warp::path::Tail| {
            let backup_domain = rustup_backup_domain2.clone();
            let work_dir2 = work_dir2.clone();
            async move {
                return_files(
                    backup_domain,
                    format!("{}/{}", "dist", tail.as_str()),
                    work_dir2,
                    PathBuf::from("dist").join(tail.as_str()),
                    false,
                )
                .await
            }
        })
        .recover(handle_missing_file);

    let work_dir3 = work_dir.clone();
    let rustup = warp::path("rustup")
        .and(warp::path::tail())
        .and_then(move |tail: warp::path::Tail| {
            let backup_domain = rustup_backup_domain.clone();
            let work_dir3 = work_dir3.clone();
            async move {
                return_files(
                    backup_domain,
                    format!("{}/{}", "rustup", tail.as_str()),
                    work_dir3,
                    PathBuf::from("rustup").join(tail.as_str()),
                    false,
                )
                .await
            }
        })
        .recover(handle_missing_file);

    let crates_backup_domain = config.crates.backup_domain.clone().unwrap_or_else(|| {
        vec![
            String::from("https://crates.rust-lang.pub"),
            String::from("localhost"),
            String::from("https://rsproxy.cn"),
            String::from("https://static.crates.io"),
        ]
    });
    let crates_1 = warp::path!("crates" / String / String / "download")
        .map(|name: String, version: String| {
            (
                format!("crates/{}/{}/download", &name, &version),
                name,
                version,
            )
        })
        .untuple_one();
    let crates_2 = warp::path!("crates" / String / String)
        .map(|name: String, file: String| {
            let split: Vec<_> = file.split('-').collect();
            let version = split[split.len() - 1].replace(".crate", "");
            (format!("crates/{}/{}", &name, &file), name, version)
        })
        .untuple_one();

    let mut work_dir4 = work_dir.clone();
    if let Some(path) = serve_index.clone() {
        work_dir4 = path.into();
    }
    let git_upload_pack = warp::path!("git-upload-pack")
        .and(warp::path::tail())
        .and(warp::method())
        .and(warp::body::aggregate())
        .and(warp::header::optional::<String>("Content-Type"))
        .and(warp::query::raw().or_else(|_| async { Ok::<(String,), Rejection>((String::new(),)) }))
        .and_then(move |_tail, method, body, content_type, _query| {
            let work_dir4 = work_dir4.clone();
            async move {
                let git_protocal = GitCommand::default();
                git_protocal
                    .git_upload_pack(body, work_dir4, method, content_type)
                    .await
            }
        });
    let mut work_dir5 = work_dir.clone();
    if let Some(path) = serve_index {
        work_dir5 = path.into();
    }
    let git_info_refs = warp::path!("info" / "refs")
        .and(warp::body::aggregate())
        .and_then(move |body| {
            let workdir = work_dir5.clone();
            async move {
                let git_protocal = GitCommand::default();
                git_protocal.git_info_refs(body, workdir).await
            }
        });

    let git = warp::path("crates.io-index").and(git_upload_pack.or(git_info_refs));

    let crates_route = crates_1
        .or(crates_2)
        .unify()
        .and_then(move |url_path: String, name: String, version: String| {
            let backup_domain = crates_backup_domain.clone();
            let work_dir2 = work_dir.clone();
            let file_path = PathBuf::from("crates")
                .join(&name)
                .join(format!("{}-{}.crate", name, version));
            async move { return_files(backup_domain, url_path, work_dir2, file_path, true).await }
        })
        .recover(handle_missing_file);
    // GET /dist/... => ./dist/..
    let routes = dist
        .or(rustup)
        .or(crates_route)
        .or(git)
        .recover(handle_rejection)
        .with(warp::trace::request());

    let (cert_path, key_path, addr, port) = (
        &file_server.cert_path,
        &file_server.key_path,
        file_server.addr,
        file_server.port,
    );

    match (cert_path, key_path) {
        (Some(cert_path), Some(key_path)) => {
            let socket_addr = parse_ipaddr(addr, port, true);
            warp::serve(routes)
                .tls()
                .cert_path(cert_path)
                .key_path(key_path)
                .run(socket_addr)
                .await;
        }
        (None, None) => {
            let socket_addr = parse_ipaddr(addr, port, false);
            warp::serve(routes).run(socket_addr).await
        }
        (Some(_), None) => {
            error!("set cert_path but not set key_path.")
        }
        (None, Some(_)) => {
            error!("set key_path but not set cert_path.")
        }
    }
}

/// parse address with ip and port
pub fn parse_ipaddr(listen: Option<IpAddr>, port: Option<u16>, use_ssl: bool) -> SocketAddr {
    let listen = listen.unwrap_or_else(|| IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
    let mut port = port.unwrap_or(8080);
    if use_ssl {
        port = 443;
    }
    SocketAddr::new(listen, port)
}

async fn download_local_files(full_path: &PathBuf) -> Result<Response<Body>, Rejection> {
    let file = File::open(full_path)
        .await
        .map_err(|_| reject::not_found())?;

    let meta = file.metadata().await.map_err(|_| reject::not_found())?;
    let stream = FramedRead::new(file, BytesCodec::new());

    let body = Body::wrap_stream(stream);

    let mut resp = Response::new(body);
    resp.headers_mut()
        .insert(http::header::CONTENT_LENGTH, meta.len().into());

    Ok(resp)
}

async fn return_files(
    backup_domain: Vec<String>,
    url_path: String,
    work_dir: PathBuf,
    file_path: PathBuf,
    is_crates: bool,
) -> Result<impl Reply, Rejection> {
    let full_path = work_dir.join(file_path.clone());
    for domain in backup_domain {
        if domain.eq("localhost") {
            info!("try to fetch file from local: {}", full_path.display());
            let res = download_local_files(&full_path).await;
            if res.is_ok() {
                return res;
            }
        } else {
            let mut uri: Uri = format!("{}/{}", domain, url_path).parse().unwrap();
            if is_crates {
                uri = format!("{}/{}", domain, file_path.display())
                    .parse()
                    .unwrap();
            }
            info!("try to fetch file from remote: {}", uri);

            let resp = reqwest::get(uri.to_string()).await.unwrap();
            if resp.status() == 200 {
                return Err(reject::custom(MissingFile { uri }));
            }
        }
    }
    Err(reject::not_found())
}

/// An API error serializable to JSON.
#[derive(Serialize)]
struct ErrorMessage {
    code: u16,
    message: String,
}

/// ### References Codes
///
/// - [warp](https://github.com/seanmonstar/warp)'s rejections (example)[https://github.com/seanmonstar/warp/blob/master/examples/rejections.rs].
///
///
// This function receives a `Rejection` and tries to return a custom
// value, otherwise simply passes the rejection along.
async fn handle_rejection(err: Rejection) -> Result<impl Reply, Infallible> {
    let code;
    let message;
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
    } else if err.find::<reject::MethodNotAllowed>().is_some() {
        // We can handle a specific error, here METHOD_NOT_ALLOWED,
        // and render it however we want
        code = StatusCode::METHOD_NOT_ALLOWED;
        message = "METHOD_NOT_ALLOWED";
    } else {
        // We should have expected this... Just log and say its a 500
        info!("unhandled rejection: {:?}", err);
        code = StatusCode::INTERNAL_SERVER_ERROR;
        message = "UNHANDLED_REJECTION";
    }

    let json = warp::reply::json(&ErrorMessage {
        code: code.as_u16(),
        message: message.into(),
    });

    Ok(warp::reply::with_status(json, code))
}

async fn handle_missing_file(err: Rejection) -> Result<impl Reply, Rejection> {
    if let Some(missing_file) = err.find::<MissingFile>() {
        let uri = missing_file.uri.clone();
        return Ok(warp::redirect::found(uri));
    }
    Err(err)
}

/// async download file from backup domain
async fn download_from_remote(path: PathBuf, uri: &Uri) -> FreightResult {
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            tokio::fs::create_dir_all(parent).await?;
        }
    }
    let mut resp = reqwest::get(uri.to_string()).await?;
    if resp.status() == 200 {
        let mut file = File::create(path).await?;
        while let Some(mut data) = resp.chunk().await? {
            file.write_all_buf(data.borrow_mut()).await?;
        }
        info!("{} {:?}", "&&&[NEW] \t\t ", file);
    } else {
        error!("download failed, Please check your url: {}", uri);
        return Err(FreighterError::code(resp.status().as_u16().into()));
    }
    Ok(())
}
