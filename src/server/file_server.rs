use bytes::{Buf, BytesMut};
use log::{error, info};
use serde::Serialize;
use std::{
    borrow::BorrowMut,
    collections::HashMap,
    convert::Infallible,
    error::Error,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
    process::Stdio,
};
use tokio::{
    fs::File,
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    process::{ChildStdout, Command},
};
use tokio_util::codec::{BytesCodec, FramedRead};
use warp::{
    http,
    http::StatusCode,
    hyper::{body::Sender, Body, Response, Uri},
    path::Tail,
    reject,
    reject::Reject,
    Filter, Rejection, Reply,
};

use crate::{config::Config, errors::FreighterError};

#[derive(Debug, PartialEq, Clone)]
struct MissingFile {
    redirect_domain: String,
    backup_domain: Vec<String>,
    url_path: String,
    full_path: PathBuf,
}

#[derive(Debug)]
pub struct FileServer {
    pub cert_path: Option<PathBuf>,
    pub key_path: Option<PathBuf>,
    pub socket_addr: SocketAddr,
}

impl Reject for MissingFile {}

/// start server
#[tokio::main]
pub async fn start(config: &Config, file_server: &FileServer) {
    tracing_subscriber::fmt::init();

    let work_dir = config.work_dir.clone().unwrap();

    let rustup_redirect_domain = config
        .rustup
        .redirect_domain
        .clone()
        .unwrap_or_else(|| String::from("https://static.rust-lang.org"));

    let rustup_backup_domain = config.rustup.backup_domain.clone().unwrap_or_else(|| {
        vec![
            String::from("https://static.rust-lang.org"),
            String::from("https://rsproxy.cn"),
        ]
    });

    let work_dir2 = work_dir.clone();
    let rustup_redirect_domain2 = rustup_redirect_domain.clone();
    let rustup_backup_domain2 = rustup_backup_domain.clone();
    let dist = warp::path("dist")
        .and(warp::path::tail())
        .and_then(move |tail: warp::path::Tail| {
            let redirect_domain = rustup_redirect_domain2.clone();
            let backup_domain = rustup_backup_domain2.clone();
            let full_path = work_dir2.join("dist").join(tail.as_str());
            async move {
                download_local_files(
                    redirect_domain,
                    backup_domain,
                    format!("{}/{}", "dist", tail.as_str()),
                    full_path,
                )
                .await
            }
        })
        .recover(handle_missing_file);

    let work_dir3 = work_dir.clone();
    let rustup = warp::path("rustup")
        .and(warp::path::tail())
        .and_then(move |tail: warp::path::Tail| {
            let redirect_domain = rustup_redirect_domain.clone();
            let backup_domain = rustup_backup_domain.clone();
            let full_path = work_dir3.join("rustup").join(tail.as_str());
            async move {
                download_local_files(
                    redirect_domain,
                    backup_domain,
                    format!("{}/{}", "rustup", tail.as_str()),
                    full_path,
                )
                .await
            }
        })
        .recover(handle_missing_file);

    let crates_redirect_domain = config
        .crates
        .redirect_domain
        .clone()
        .unwrap_or_else(|| String::from("https://rsproxy.cn"));

    let crates_backup_domain = config.crates.backup_domain.clone().unwrap_or_else(|| {
        vec![
            String::from("https://rsproxy.cn"),
            String::from("https://static.crates.io"),
        ]
    });
    let crates_1 = warp::path!("crates" / String / String / "download")
        .map(|name: String, version: String| {
            (
                format!("crates/{}/{}download", &name, &version),
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

    let work_dir4 = work_dir.clone();
    let git = warp::path("crates.io-index")
        .and(warp::path::tail())
        .and(warp::method())
        .and(warp::body::aggregate())
        .and(warp::header::optional::<String>("Content-Type"))
        .and(warp::query::raw().or_else(|_| async { Ok::<(String,), Rejection>((String::new(),)) }))
        .and_then(move |tail, method, body, content_type, query| {
            let workdir = work_dir4.clone();
            async move { fetch_git(workdir, tail, method, query, content_type, body).await }
        });

    let crates_route = crates_1
        .or(crates_2)
        .unify()
        .and_then(move |url_path: String, name: String, version: String| {
            let redirect_domain = crates_redirect_domain.clone();
            let backup_domain = crates_backup_domain.clone();
            let full_path = work_dir
                .join("crates")
                .join(&name)
                .join(format!("{}-{}.crate", name, version));
            async move {
                download_local_files(redirect_domain, backup_domain, url_path, full_path).await
            }
        })
        .recover(handle_missing_file);
    // GET /dist/... => ./dist/..
    let routes = dist
        .or(rustup)
        .or(crates_route)
        .or(git)
        .recover(handle_rejection)
        .with(warp::trace::request());

    let (cert_path, key_path, socket_addr) = (
        &file_server.cert_path,
        &file_server.key_path,
        file_server.socket_addr,
    );

    match (cert_path, key_path) {
        (Some(cert_path), Some(key_path)) => {
            warp::serve(routes)
                .tls()
                .cert_path(cert_path)
                .key_path(key_path)
                .run(socket_addr)
                .await;
        }
        (None, None) => warp::serve(routes).run(socket_addr).await,
        (Some(_), None) => {
            error!("set cert_path but not set key_path.")
        }
        (None, Some(_)) => {
            error!("set key_path but not set cert_path.")
        }
    }
}

/// parse address with ip and port
pub fn parse_ipaddr(listen: Option<IpAddr>, port: Option<u16>) -> SocketAddr {
    let listen = listen.unwrap_or_else(|| IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
    let port = port.unwrap_or(8080);
    SocketAddr::new(listen, port)
}

async fn download_local_files(
    redirect_domain: String,
    backup_domain: Vec<String>,
    url_path: String,
    full_path: PathBuf,
) -> Result<Response<Body>, Rejection> {
    let missing_file = &MissingFile {
        redirect_domain,
        backup_domain,
        url_path,
        full_path: full_path.clone(),
    };

    let file = File::open(full_path)
        .await
        .map_err(|_| reject::custom(missing_file.to_owned()))?;

    let meta = file
        .metadata()
        .await
        .map_err(|_| reject::custom(missing_file.to_owned()))?;
    let stream = FramedRead::new(file, BytesCodec::new());

    let body = Body::wrap_stream(stream);

    let mut resp = Response::new(body);
    resp.headers_mut()
        .insert(http::header::CONTENT_LENGTH, meta.len().into());

    Ok(resp)
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
    } else if err.find::<warp::reject::MethodNotAllowed>().is_some() {
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
        let (redirect_domain, _, url_path, full_path) = (
            &missing_file.redirect_domain,
            &missing_file.backup_domain,
            &missing_file.url_path,
            &missing_file.full_path,
        );
        info!("{:?}", &missing_file);
        let uri: Uri = format!("{}/{}", redirect_domain, url_path).parse().unwrap();
        info!("can't found local file, redirect to : {}", uri);

        download_from_remote(full_path.to_owned(), &uri)
            .await
            .unwrap();

        return Ok(warp::redirect::found(uri));
    }
    Err(err)
}

/// ### References Codes
///
/// - [conduit-git-http-backend][https://github.com/conduit-rust/conduit-git-http-backend/blob/master/src/lib.rs].
///
///
/// hanlde request from git client
async fn fetch_git(
    work_dir: PathBuf,
    tail: Tail,
    method: http::Method,
    query: String,
    content_type: Option<String>,
    mut body: impl Buf,
) -> Result<impl Reply, Rejection> {
    let mut cmd = Command::new("git");
    cmd.arg("http-backend");
    cmd.env("GIT_PROJECT_ROOT", &work_dir);
    cmd.env("PATH_INFO", format!("/crates.io-index/{}", tail.as_str()));
    cmd.env("REQUEST_METHOD", method.as_str());
    cmd.env("QUERY_STRING", query);
    if let Some(content_type) = content_type {
        cmd.env("CONTENT_TYPE", content_type);
    }
    cmd.env("GIT_HTTP_EXPORT_ALL", "true");
    cmd.stderr(Stdio::inherit());
    cmd.stdout(Stdio::piped());
    cmd.stdin(Stdio::piped());

    let p = cmd.spawn().unwrap();
    let mut git_input = p.stdin.unwrap();

    while body.has_remaining() {
        git_input.write_all_buf(&mut body.chunk()).await.unwrap();
        let cnt = body.chunk().len();
        body.advance(cnt);
    }

    let mut git_output = BufReader::new(p.stdout.unwrap());
    let mut headers = HashMap::new();
    loop {
        let mut line = String::new();
        git_output.read_line(&mut line).await.unwrap();
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some((key, value)) = line.split_once(": ") {
            headers.insert(key.to_string(), value.to_string());
        }
    }

    let mut resp = Response::builder();
    for (key, val) in headers {
        resp = resp.header(&key, val);
    }

    let (sender, body) = Body::channel();
    tokio::spawn(send_git(sender, git_output));
    let resp = resp.body(body).unwrap();
    Ok(resp)
}

async fn send_git(
    mut sender: Sender,
    mut git_output: BufReader<ChildStdout>,
) -> Result<(), FreighterError> {
    loop {
        let mut bytes_out = BytesMut::new();
        git_output.read_buf(&mut bytes_out).await?;
        if bytes_out.is_empty() {
            return Ok(());
        }
        sender.send_data(bytes_out.freeze()).await.unwrap();
    }
}

/// async download file from backup domain
async fn download_from_remote(path: PathBuf, uri: &Uri) -> Result<bool, FreighterError> {
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            tokio::fs::create_dir_all(parent).await?;
        }
    }
    let mut resp = reqwest::get(uri.to_string()).await?;
    if resp.status() == 200 {
        let mut file = tokio::fs::File::create(path).await?;
        while let Some(mut data) = resp.chunk().await? {
            file.write_all_buf(data.borrow_mut()).await?;
        }
        info!("{} {:?}", "&&&[NEW] \t\t ", file);
    } else {
        error!("download failed, Please check your url: {}", uri);
        return Ok(false);
    }
    Ok(true)
}
