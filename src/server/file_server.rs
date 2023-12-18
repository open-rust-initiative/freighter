//!
//!
//!
//!
//!
//!

use std::{
    net::{IpAddr, SocketAddr},
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
    pub addr: IpAddr,
    pub port: u16,
}

/// start server
#[tokio::main]
pub async fn start(config: &Config, file_server: &FileServer) {
    tracing_subscriber::fmt::init();
    // storage::init().await;
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
            let socket_addr = SocketAddr::new(addr, port);
            warp::serve(routes)
                .tls()
                .cert_path(cert_path)
                .key_path(key_path)
                .run(socket_addr)
                .await;
        }
        (None, None) => {
            let socket_addr = SocketAddr::new(addr, port);
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

    use bytes::{Buf, Bytes};
    use warp::{Filter, Rejection};

    use crate::{
        config::Config,
        server::{
            file_server::utils,
            git_protocol::GitCommand,
            model::{CratesPublish, Errors, PublishRsp},
        },
    };

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
            .or(crates(config.clone()))
            .or(git(git_work_dir))
            .or(publish(config.clone()))
            .or(sparse_index(config))
    }

    pub fn publish(
        config: Config,
    ) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
        warp::path!("api" / "v1" / "crates" / "new")
            .and(warp::body::bytes())
            .and(with_config(config))
            .map(|mut body: Bytes, config: Config| {
                let json_len = utils::get_usize_from_bytes(body.copy_to_bytes(4));

                tracing::info!("json_len: {:?}", json_len);
                let json = body.copy_to_bytes(json_len);
                tracing::info!("raw json: {:?}", json);

                let parse_result = serde_json::from_slice::<CratesPublish>(json.as_ref());
                let crate_len = utils::get_usize_from_bytes(body.copy_to_bytes(4));
                let file_content = body.copy_to_bytes(crate_len);

                match parse_result {
                    Ok(result) => {
                        println!("JSON: {:?}", result);
                        let work_dir = config.work_dir.unwrap();
                        utils::save_crate_index(
                            &result,
                            &file_content,
                            work_dir.join("crates.io-index"),
                        );
                        utils::save_crate_file(&result, &file_content, work_dir.join("crates"));
                        // let std::fs::write();
                        // 1.verify name and version from local db
                        // 2.call remote server to check info in crates.io
                        warp::reply::json(&PublishRsp::default())
                    }
                    Err(err) => warp::reply::json(&Errors::new(err.to_string())),
                }
            })
    }

    pub fn sparse_index(
        config: Config,
    ) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
        warp::path("index")
            .and(warp::path::tail())
            .and(with_config(config))
            .and_then(|tail: warp::path::Tail, config: Config| async move {
                handlers::return_files(
                    config.rustup.serve_domains.unwrap(),
                    config.work_dir.unwrap(),
                    PathBuf::from("crates.io-index").join(tail.as_str()),
                    false,
                )
                .await
            })
    }

    // build '/dist/*' route, this route handle rust toolchian files request
    pub fn dist(
        config: Config,
    ) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
        warp::path("dist")
            .and(warp::path::tail())
            .and(with_config(config))
            .and_then(|tail: warp::path::Tail, config: Config| async move {
                handlers::return_files(
                    config.rustup.serve_domains.unwrap(),
                    config.work_dir.unwrap(),
                    PathBuf::from("dist").join(tail.as_str()),
                    false,
                )
                .await
            })
            .recover(handlers::handle_missing_file)
    }

    // build '/rustup/*' route, this route handle rustup-init file request
    pub fn rustup(
        config: Config,
    ) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
        warp::path("rustup")
            .and(warp::path::tail())
            .and(with_config(config))
            .and_then(move |tail: warp::path::Tail, config: Config| async move {
                handlers::return_files(
                    config.rustup.serve_domains.unwrap(),
                    config.work_dir.unwrap(),
                    PathBuf::from("rustup").join(tail.as_str()),
                    false,
                )
                .await
            })
            .recover(handlers::handle_missing_file)
    }

    // build '/crates/*' route, this route handle crates file request
    pub fn crates(
        config: Config,
    ) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
        let crates_1 = warp::path!("crates" / String / String / "download")
            .map(|name: String, version: String| (name, version))
            .untuple_one();
        let crates_2 = warp::path!("crates" / String / String)
            .map(|name: String, file: String| {
                let split: Vec<_> = file.split('-').collect();
                let version = split[split.len() - 1].replace(".crate", "");
                (name, version)
            })
            .untuple_one();

        crates_1
            .or(crates_2)
            .unify()
            .and(with_config(config))
            .and_then(|name: String, version: String, config: Config| async move {
                let file_path = PathBuf::from("crates")
                    .join(&name)
                    .join(format!("{}-{}.crate", name, version));
                handlers::return_files(
                    config.crates.serve_domains.unwrap(),
                    config.work_dir.unwrap(),
                    file_path,
                    true,
                )
                .await
            })
            .recover(handlers::handle_missing_file)
    }

    // build '/crate.io-index/(git protocol)' route, this route handle gti clone and git pull request
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
    use std::{borrow::BorrowMut, convert::Infallible, error::Error, path::PathBuf, str::FromStr};

    use reqwest::Url;
    use serde::Serialize;
    use tokio::{fs::File, io::AsyncWriteExt};
    use tokio_util::codec::{BytesCodec, FramedRead};
    use url::form_urlencoded::byte_serialize;
    use warp::{
        http,
        http::StatusCode,
        hyper::{Body, Response, Uri},
        reject, Rejection, Reply,
    };

    use crate::{
        download,
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
        work_dir: PathBuf,
        mut file_path: PathBuf,
        is_crates: bool,
    ) -> Result<impl Reply, Rejection> {
        for domain in serve_domains {
            if domain.eq("localhost") {
                let full_path = work_dir.join(file_path.clone());
                tracing::info!("try to fetch file from local: {}", full_path.display());
                let res = download_local_files(&full_path).await;
                if res.is_ok() {
                    return res;
                }
            } else {
                // url_path:  crates/name/version/download or crates/name/version
                // file_path: crates/name/name-version.crate
                let mut url: Url = format!("{}/{}", domain, file_path.display())
                    .parse()
                    .unwrap();
                if is_crates && domain.contains("myhuaweicloud.com") {
                    download::encode_huaweicloud_url(&mut url);

                    let name = file_path.file_name().unwrap().to_str().unwrap();
                    let encode: String = byte_serialize(name.as_bytes()).collect();
                    file_path.pop();
                    file_path.push(encode);
                    tracing::debug!("file path {:?}", file_path);
                }
                return Ok(
                    warp::redirect::found(Uri::from_str(url.as_str()).unwrap()).into_response()
                );
                // return Err(reject::custom(MissingFile { uri }));
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
    /// - [warp](<https://github.com/seanmonstar/warp>)'s rejections (example)[<https://github.com/seanmonstar/warp/blob/master/examples/rejections.rs>].
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
            tracing::info!("redirect to: {}", uri);
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

mod utils {
    use std::{fs, path::PathBuf};

    use crate::{
        handler::{crates_file::IndexFile, utils},
        server::model::CratesPublish,
    };
    use bytes::Bytes;
    use sha2::{Digest, Sha256};

    pub fn get_usize_from_bytes(bytes: Bytes) -> usize {
        let mut fixed_array = [0u8; 8];
        fixed_array[..4].copy_from_slice(&bytes[..4]);
        usize::from_le_bytes(fixed_array)
    }

    pub fn save_crate_index(json: &CratesPublish, content: &Bytes, work_dir: PathBuf) {
        let suffix = utils::index_suffix(&json.name);
        let index_path = work_dir.join(suffix);
        //convert publish json to index file
        let mut index_file: IndexFile =
            serde_json::from_str(&serde_json::to_string(&json).unwrap()).unwrap();

        let mut hasher = Sha256::new();
        hasher.update(content);
        index_file.cksum = Some(format!("{:x}", hasher.finalize()));
        fs::write(index_path, serde_json::to_string(&index_file).unwrap()).unwrap();
    }

    pub fn save_crate_file(json: &CratesPublish, content: &Bytes, work_dir: PathBuf) {
        let crates_dir = work_dir.join(&json.name);
        if !crates_dir.exists() {
            fs::create_dir_all(&crates_dir).unwrap();
        }
        let crates_file = crates_dir.join(format!("{}-{}.crate", json.name, json.vers));
        fs::write(crates_file, content).unwrap();
    }
}
