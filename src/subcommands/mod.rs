pub mod io;
pub mod ops;

pub use io::{Arg, Rtn, Storage};
pub use ops::{
    AddSubCommand, CliSubCommand, DelSubCommand, ListSubCommand, NowSubCommand, UpdateSubCommand,
};

use structopt::clap::AppSettings;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(global_settings = &[AppSettings::ColoredHelp, AppSettings::VersionlessSubcommands])]
pub struct Cli {
    #[structopt(long = "mode", help = "storage mode: db or file", default_value = "db")]
    pub mode: String,

    #[structopt(
        long = "file-path",
        help = "path to json file (required when mode is file)"
    )]
    pub file_path: Option<String>,

    #[structopt(subcommand)]
    pub command: Command,
}

#[derive(StructOpt, Debug)]
pub enum Command {
    #[structopt(name = "add", about = "Create new account")]
    Add {
        #[structopt(short = "e", long = "exchange", help = "exchange name")]
        exchange: String,
        #[structopt(short = "n", long = "name", help = "account name")]
        name: String,
        #[structopt(short = "s", long = "secret", help = "secret key")]
        secret: String,
    },

    #[structopt(name = "delete", about = "Delete new account")]
    Delete {
        #[structopt(short = "e", long = "exchange", help = "exchange name")]
        exchange: String,
        #[structopt(short = "n", long = "name", help = "account name")]
        name: String,
    },

    #[structopt(name = "update", about = "Update new account")]
    Update {
        #[structopt(short = "e", long = "exchange", help = "exchange name")]
        exchange: String,
        #[structopt(short = "n", long = "name", help = "account name")]
        name: String,
        #[structopt(short = "s", long = "secret", help = "secret key")]
        secret: String,
    },

    #[structopt(name = "list", about = "List all accounts")]
    List {
        #[structopt(short = "e", long = "exchange", help = "exchange name")]
        exchange: Option<String>,
    },

    #[structopt(name = "now", about = "Show account GA code")]
    Now {
        #[structopt(short = "e", long = "exchange", help = "exchange name")]
        exchange: String,
        #[structopt(short = "n", long = "name", help = "account name")]
        name: String,
    },
}

pub fn process(storage: &dyn Storage, command: Command) -> Result<String, String> {
    let result = match command {
        Command::Add {
            exchange,
            name,
            secret,
        } => AddSubCommand { storage }.process(Arg {
            exchange: exchange,
            name: name,
            secret: Some(secret),
        }),

        Command::Delete { exchange, name } => DelSubCommand { storage }.process(Arg {
            exchange: exchange,
            name: name,
            secret: None,
        }),

        Command::Update {
            exchange,
            name,
            secret,
        } => UpdateSubCommand { storage }.process(Arg {
            exchange: exchange,
            name: name,
            secret: Some(secret),
        }),

        Command::List { exchange } => ListSubCommand { storage }.process(exchange),

        Command::Now { exchange, name } => NowSubCommand { storage }.process(Arg {
            exchange: exchange,
            name: name,
            secret: None,
        }),
    };

    result.map(|rtn| format!("{}", rtn))
}
