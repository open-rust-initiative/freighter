///
///
///
///
///


use crate::config::Config;
use crate::errors::FreightResult;

pub type App = clap::Command<'static>;

pub fn main(_config: &mut Config) -> FreightResult {
    println!("Hello, world!");

    Ok(())
}

fn cli() -> App {
    App::new("freight")
        .version("0.1.0")
        .author("Open Rust Initiative")
}