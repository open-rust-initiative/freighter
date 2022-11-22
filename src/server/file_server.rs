use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
};

use warp::Filter;

pub async fn start(path: PathBuf, socket_addr: SocketAddr) {
    let dist = warp::path("dist").and(warp::fs::dir(path.join("dist")));
    let rustup = warp::path("rustup").and(warp::fs::dir(path.join("rustup")));

    // GET /dist/... => ./dist/..
    let routes = dist.or(rustup);

    warp::serve(routes).run(socket_addr).await;
}

pub fn parse_ipaddr(listen: Option<IpAddr>, port: Option<u16>) -> SocketAddr {
    let listen = listen.unwrap_or_else(|| IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
    let port = port.unwrap_or(8080);
    SocketAddr::new(listen, port)
}
