#![allow(incomplete_features)]
use bytes::{Buf, BufMut, BytesMut};
use log::info;
use std::{collections::HashMap, path::PathBuf, process::Stdio};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    process::{ChildStdout, Command},
};

use warp::{
    http,
    hyper::{body::Sender, Body, Response},
    Rejection,
};

use crate::errors::FreighterError;

/// see https://git-scm.com/docs/gitprotocol-http
pub trait GitProtocal {
    /// Discovering References:
    /// All HTTP clients MUST begin either a fetch or a push exchange by discovering the references available on the remote repository.
    async fn git_info_refs(
        &self,
        body: impl Buf,
        work_dir: PathBuf,
    ) -> Result<Response<Body>, Rejection>;

    /// Smart Service git-upload-pack
    async fn git_upload_pack(
        &self,
        body: impl Buf,
        work_dir: PathBuf,
        method: http::Method,
        content_type: Option<String>,
    ) -> Result<Response<Body>, Rejection>;
}

#[derive(Default)]
pub struct GitCommand {}


/// ### References Codes
///
/// - [conduit-git-http-backend][https://github.com/conduit-rust/conduit-git-http-backend/blob/master/src/lib.rs].
///
///
/// hanlde request from git client
impl GitProtocal for GitCommand {
    async fn git_info_refs(
        &self,
        mut body: impl Buf,
        work_dir: PathBuf,
    ) -> Result<Response<Body>, Rejection> {
        let mut cmd = Command::new("git");
        // git 数据检查
        cmd.args([
            "upload-pack",
            // "--http-backend-info-refs",
            "--stateless-rpc",
            "--advertise-refs",
            work_dir.join("crates.io-index").to_str().unwrap(),
        ]);
        cmd.stdin(Stdio::piped()).stdout(Stdio::piped());

        let p = cmd.spawn().unwrap();
        let mut git_input = p.stdin.unwrap();

        while body.has_remaining() {
            git_input.write_all_buf(&mut body.chunk()).await.unwrap();

            let cnt = body.chunk().len();
            body.advance(cnt);
        }
        let git_output = BufReader::new(p.stdout.unwrap());
        let mut headers = HashMap::new();
        headers.insert(
            "Content-Type".to_string(),
            "application/x-git-upload-pack-advertisement".to_string(),
        );
        headers.insert(
            "Cache-Control".to_string(),
            "no-cache, max-age=0, must-revalidate".to_string(),
        );
        info!("headers: {:?}", headers);
        let mut resp = Response::builder();
        for (key, val) in headers {
            resp = resp.header(&key, val);
        }

        let (sender, body) = Body::channel();
        tokio::spawn(send(sender, git_output, true));

        let resp = resp.body(body).unwrap();
        Ok(resp)
    }

    async fn git_upload_pack(
        &self,
        mut body: impl Buf,
        work_dir: PathBuf,
        method: http::Method,
        content_type: Option<String>,
    ) -> Result<Response<Body>, Rejection> {
        let mut cmd = Command::new("git");
        cmd.arg("http-backend");
        cmd.env("GIT_PROJECT_ROOT", &work_dir);
        cmd.env("PATH_INFO", "/crates.io-index/git-upload-pack");
        cmd.env("REQUEST_METHOD", method.as_str());
        // cmd.env("QUERY_STRING", query);
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
            println!(
                "request body: {:?}",
                String::from_utf8(body.chunk().to_vec()).unwrap()
            );
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
        info!("headers: {:?}", headers);
        let mut resp = Response::builder();
        for (key, val) in headers {
            resp = resp.header(&key, val);
        }

        let (sender, body) = Body::channel();
        tokio::spawn(send(sender, git_output, false));
        let resp = resp.body(body).unwrap();
        Ok(resp)
    }
}

async fn send(
    mut sender: Sender,
    mut git_output: BufReader<ChildStdout>,
    add_refs: bool,
) -> Result<(), FreighterError> {
    if add_refs {
        let mut buf = BytesMut::new();
        buf.put(&b"001e# service=git-upload-pack\n0000"[..]);
        sender.send_data(buf.freeze()).await.unwrap();
    }

    loop {
        let mut bytes_out = BytesMut::new();
        git_output.read_buf(&mut bytes_out).await?;
        if bytes_out.is_empty() {
            println!("send:empty");
            return Ok(());
        }
        if add_refs {
            println!("send: bytes_out: {:?}", bytes_out.clone().freeze());
        }
        sender.send_data(bytes_out.freeze()).await.unwrap();
    }
}
