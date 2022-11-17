//!
//!
//!
//!
//!

use std::path::PathBuf;
use std::str::FromStr;

use clap::{arg, ArgMatches};
use log::LevelFilter;
use log4rs::append::console::ConsoleAppender;
use log4rs::append::file::FileAppender;
use log4rs::config::runtime::Config as Log4rsConfig;
use log4rs::config::Logger;
use log4rs::config::{Appender, Root};
use log4rs::encode::pattern::PatternEncoder;

use crate::commands;
use crate::config::{Config, LogConfig};
use crate::errors::{FreightResult, FreighterError};

///
///
///
///
///

pub type App = clap::Command;

///
///
pub fn main(config: &mut Config) -> FreightResult {
    // log4rs::init_file("log4rs.yaml", Default::default()).unwrap();

    let args = cli().try_get_matches()?;

    let work_dir = match args.get_one::<String>("work-dir").cloned() {
        Some(work_dir) => PathBuf::from(work_dir),
        None => dirs::home_dir().unwrap(),
    };
    let mut config = config.load(&work_dir);
    config.work_dir = Some(work_dir.clone());
    init_log(&config.log, work_dir).unwrap();

    let (cmd, subcommand_args) = match args.subcommand() {
        Some((cmd, args)) => (cmd, args),
        _ => {
            // No subcommand provided.
            cli().print_help()?;
            return Ok(());
        }
    };

    execute_subcommand(&mut config, cmd, subcommand_args)
}

///
///
fn cli() -> App {
    let usage = "freight [SUBCOMMAND]";

    App::new("freight")
        .version("0.1.0")
        .disable_colored_help(true)
        .disable_help_subcommand(true)
        .override_usage(usage)
        .author("Open Rust Initiative")
        .arg(arg!(-c --"work-dir" <FILE> "specify the work dir,
             where to downlaod crates, rust toolchains and storage logs, default: $HOME/.freighter")
        )
        .help_template(
            "\
Freight - A crate registry from the Open Rust Initiative Community

USAGE:
    {usage}

Some common freight commands are (see all commands with --list):
    sync    Sync the index and crate files from the upstream to local, cloud or registry

See 'freight help <command>' for more information on a specific command.\n"
        )
        .subcommands(commands::builtin())
}

///
///
pub fn execute_subcommand(config: &mut Config, cmd: &str, args: &ArgMatches) -> FreightResult {
    if let Some(f) = commands::builtin_exec(cmd) {
        f(config, args)
    } else {
        Err(FreighterError::unknown_command(cmd.to_string()))
    }
}

pub fn init_log(config: &LogConfig, work_dir: PathBuf) -> FreightResult {
    let binding = work_dir.join("freighter/log/info.log");
    let log_path = binding.to_str().unwrap();
    let level = LevelFilter::from_str(&config.level).unwrap();


    let encoder = PatternEncoder::new("{d}:{l} - {m}{n}");

    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(encoder.clone()))
        .build();

    // let policy = Policy::builder().build();
    let file = FileAppender::builder()
        .encoder(Box::new(encoder))
        .build(log_path)
        .unwrap();

    let log4rs_config = Log4rsConfig::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .appender(Appender::builder().build("file", Box::new(file)))
        .logger(
            Logger::builder()
                .appender("file")
                .additive(false)
                .build("app::file", level),
        )
        .build(
            Root::builder()
                .appender("stdout")
                .appender("file")
                .build(level),
        )
        .unwrap();

    log4rs::init_config(log4rs_config).unwrap();

    Ok(())
}
