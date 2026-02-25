#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate google_authenticator;
extern crate r2d2_redis;

mod db;
mod file;
mod subcommands;

use std::path::PathBuf;

use db::{r2d2, RedisConnectionManager, DB};
use file::FileDB;
use structopt::StructOpt;
use subcommands::Cli;

fn expand_tilde(path: &str) -> PathBuf {
    if path.starts_with("~/") {
        match dirs::home_dir() {
            Some(home) => home.join(&path[2..]),
            None => PathBuf::from(path),
        }
    } else {
        PathBuf::from(path)
    }
}

fn main() {
    let cli = Cli::from_args();

    let result = match cli.mode.as_str() {
        "file" => {
            let storage = FileDB {
                file_path: expand_tilde(&cli.file_path),
            };
            subcommands::process(&storage, cli.command)
        }
        "db" => {
            let manager = RedisConnectionManager::new("redis://localhost").unwrap();
            let pool = r2d2::Pool::builder().build(manager).unwrap();
            let storage = DB {
                db_name: "cliotp",
                pool: &pool,
            };
            subcommands::process(&storage, cli.command)
        }
        other => Err(format!("unknown mode: {}, use 'file' or 'db'", other)),
    };

    match result {
        Ok(output) => println!("{}", output),
        Err(e) => {
            println!("{:?}", e);
            std::process::exit(1);
        }
    }
}
