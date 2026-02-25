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

fn main() {
    let cli = Cli::from_args();

    let result = match cli.mode.as_str() {
        "db" => {
            let manager = RedisConnectionManager::new("redis://localhost").unwrap();
            let pool = r2d2::Pool::builder().build(manager).unwrap();
            let storage = DB {
                db_name: "cliotp",
                pool: &pool,
            };
            subcommands::process(&storage, cli.command)
        }
        "file" => {
            let path = cli
                .file_path
                .ok_or(String::from("--file-path is required when mode is file"));
            match path {
                Ok(p) => {
                    let storage = FileDB {
                        file_path: PathBuf::from(p),
                    };
                    subcommands::process(&storage, cli.command)
                }
                Err(e) => Err(e),
            }
        }
        other => Err(format!("unknown mode: {}, use 'db' or 'file'", other)),
    };

    match result {
        Ok(output) => println!("{}", output),
        Err(e) => {
            println!("{:?}", e);
            std::process::exit(1);
        }
    }
}
