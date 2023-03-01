//!
//!
//!
//!
//!
//!

use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
};

use warp::{hyper::Uri, reject::Reject, Filter};

use crate::config::Config;

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
    let routes = filters::build_route(config.to_owned())
        .recover(handlers::handle_rejection)
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
            tracing::error!("set cert_path but not set key_path.")
        }
        (None, Some(_)) => {
            tracing::error!("set key_path but not set cert_path.")
        }
    }
}
mod filters {
    use std::path::PathBuf;

    use warp::{Filter, Rejection};

    use crate::{config::Config, server::git_protocal::GitCommand};

    use super::handlers;

    pub fn build_route(
        config: Config,
    ) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
        let git_work_dir = if let Some(path) = &config.crates.serve_index {
            PathBuf::from(path)
        } else {
            config.work_dir.clone().unwrap()
        };

        // GET /dist/... => ./dist/..
        dist(config.clone())
            .or(rustup(config.clone()))
            .or(crates(config))
            .or(git(git_work_dir))
    }

    pub fn dist(
        config: Config,
    ) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
        warp::path("dist")
            .and(warp::path::tail())
            .and(with_config(config))
            .and_then(|tail: warp::path::Tail, config: Config| async move {
                handlers::return_files(
                    config.rustup.serve_domains.unwrap(),
                    format!("{}/{}", "dist", tail.as_str()),
                    config.work_dir.unwrap(),
                    PathBuf::from("dist").join(tail.as_str()),
                    false,
                )
                .await
            })
            .recover(handlers::handle_missing_file)
    }

    pub fn rustup(
        config: Config,
    ) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
        warp::path("rustup")
            .and(warp::path::tail())
            .and(with_config(config))
            .and_then(move |tail: warp::path::Tail, config: Config| async move {
                handlers::return_files(
                    config.rustup.serve_domains.unwrap(),
                    format!("{}/{}", "rustup", tail.as_str()),
                    config.work_dir.unwrap(),
                    PathBuf::from("rustup").join(tail.as_str()),
                    false,
                )
                .await
            })
            .recover(handlers::handle_missing_file)
    }

    pub fn crates(
        config: Config,
    ) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
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

        crates_1
            .or(crates_2)
            .unify()
            .and(with_config(config))
            .and_then(
                |url_path: String, name: String, version: String, config: Config| async move {
                    let file_path = PathBuf::from("crates")
                        .join(&name)
                        .join(format!("{}-{}.crate", name, version));
                    handlers::return_files(
                        config.crates.serve_domains.unwrap(),
                        url_path,
                        config.work_dir.unwrap(),
                        file_path,
                        true,
                    )
                    .await
                },
            )
            .recover(handlers::handle_missing_file)
    }

    pub fn git(
        git_work_dir: PathBuf,
    ) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
        let git_upload_pack = warp::path!("git-upload-pack")
            .and(warp::path::tail())
            .and(warp::method())
            .and(warp::body::aggregate())
            .and(warp::header::optional::<String>("Content-Type"))
            .and(
                warp::query::raw()
                    .or_else(|_| async { Ok::<(String,), Rejection>((String::new(),)) }),
            )
            .and(with_work_dir(git_work_dir.to_owned()))
            .and_then(
                |_tail, method, body, content_type, _query, work_dir| async move {
                    let git_protocal = GitCommand::default();
                    git_protocal
                        .git_upload_pack(body, work_dir, method, content_type)
                        .await
                },
            );

        let git_info_refs = warp::path!("info" / "refs")
            .and(warp::body::aggregate())
            .and(with_work_dir(git_work_dir))
            .and_then(|body, work_dir| async move {
                let git_protocal = GitCommand::default();
                git_protocal.git_info_refs(body, work_dir).await
            });

        warp::path("crates.io-index").and(git_upload_pack.or(git_info_refs))
    }

    fn with_config(
        config: Config,
    ) -> impl Filter<Extract = (Config,), Error = std::convert::Infallible> + Clone {
        warp::any().map(move || config.clone())
    }

    fn with_work_dir(
        work_dir: PathBuf,
    ) -> impl Filter<Extract = (PathBuf,), Error = std::convert::Infallible> + Clone {
        warp::any().map(move || work_dir.clone())
    }
}

mod handlers {
    use std::{borrow::BorrowMut, convert::Infallible, error::Error, path::PathBuf};

    use serde::Serialize;
    use tokio::{fs::File, io::AsyncWriteExt};
    use tokio_util::codec::{BytesCodec, FramedRead};
    use warp::{
        http,
        http::StatusCode,
        hyper::{Body, Response, Uri},
        reject, Rejection, Reply,
    };

    use crate::{
        errors::{FreightResult, FreighterError},
        server::file_server::MissingFile,
    };

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

    pub async fn return_files(
        serve_domains: Vec<String>,
        url_path: String,
        work_dir: PathBuf,
        file_path: PathBuf,
        is_crates: bool,
    ) -> Result<impl Reply, Rejection> {
        let full_path = work_dir.join(file_path.clone());
        for domain in serve_domains {
            if domain.eq("localhost") {
                tracing::info!("try to fetch file from local: {}", full_path.display());
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
                tracing::info!("try to fetch file from remote: {}", uri);

                let resp = reqwest::get(uri.to_string()).await.unwrap();
                if resp.status().is_success() {
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
    pub async fn handle_rejection(err: Rejection) -> Result<impl Reply, Infallible> {
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
            tracing::info!("unhandled rejection: {:?}", err);
            code = StatusCode::INTERNAL_SERVER_ERROR;
            message = "UNHANDLED_REJECTION";
        }

        let json = warp::reply::json(&ErrorMessage {
            code: code.as_u16(),
            message: message.into(),
        });

        Ok(warp::reply::with_status(json, code))
    }

    pub async fn handle_missing_file(err: Rejection) -> Result<impl Reply, Rejection> {
        if let Some(missing_file) = err.find::<MissingFile>() {
            let uri = missing_file.uri.clone();
            return Ok(warp::redirect::found(uri));
        }
        Err(err)
    }

    #[allow(unused)]
    /// async download file from backup domain
    pub async fn download_from_remote(path: PathBuf, uri: &Uri) -> FreightResult {
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
            tracing::info!("{} {:?}", "&&&[NEW] \t\t ", file);
        } else {
            tracing::error!("download failed, Please check your url: {}", uri);
            return Err(FreighterError::code(resp.status().as_u16().into()));
        }
        Ok(())
    }
}

/// parse address with ip and port
fn parse_ipaddr(listen: Option<IpAddr>, port: Option<u16>, use_ssl: bool) -> SocketAddr {
    let listen = listen.unwrap_or_else(|| IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
    let mut port = port.unwrap_or(8080);
    if use_ssl {
        port = 443;
    }
    SocketAddr::new(listen, port)
}
