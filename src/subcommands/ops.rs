use google_authenticator::GA_AUTH;

use std::fs;
use std::path::PathBuf;

use super::io::{parse_data, Arg, Data, Rtn, Storage};

pub trait CliSubCommand {
    fn process(&self, arg: Arg) -> Result<Rtn, String>;
}

pub struct AddSubCommand<'a> {
    pub storage: &'a dyn Storage,
}

impl<'a> CliSubCommand for AddSubCommand<'a> {
    fn process(&self, arg: Arg) -> Result<Rtn, String> {
        self.storage.add(&arg)
    }
}

pub struct DelSubCommand<'a> {
    pub storage: &'a dyn Storage,
}

impl<'a> CliSubCommand for DelSubCommand<'a> {
    fn process(&self, arg: Arg) -> Result<Rtn, String> {
        self.storage.delete(&arg)
    }
}

pub struct ListSubCommand<'a> {
    pub storage: &'a dyn Storage,
}

impl<'a> ListSubCommand<'a> {
    pub fn process(&self, exchange: Option<String>) -> Result<Rtn, String> {
        self.storage.list(exchange)
    }
}

pub struct UpdateSubCommand<'a> {
    pub storage: &'a dyn Storage,
}

impl<'a> CliSubCommand for UpdateSubCommand<'a> {
    fn process(&self, arg: Arg) -> Result<Rtn, String> {
        self.storage.update(&arg)
    }
}

pub struct NowSubCommand<'a> {
    pub storage: &'a dyn Storage,
}

impl<'a> CliSubCommand for NowSubCommand<'a> {
    fn process(&self, arg: Arg) -> Result<Rtn, String> {
        self.storage.get(&arg).and_then(|rtn| match rtn {
            Rtn::Secret { secret } => get_code!(&secret)
                .map_err(|e| format!("{:?}", e))
                .map(|code| Rtn::Code { code }),
            _ => Ok(Rtn::Empty),
        })
    }
}

pub struct MergeSubCommand<'a> {
    pub storage: &'a dyn Storage,
}

impl<'a> MergeSubCommand<'a> {
    pub fn process(&self, files: Vec<PathBuf>) -> Result<Rtn, String> {
        let mut combined: Data = std::collections::HashMap::new();
        for file_path in &files {
            let content = fs::read_to_string(file_path)
                .map_err(|e| format!("failed to read {}: {:?}", file_path.display(), e))?;
            let data = parse_data(&content)
                .map_err(|e| format!("failed to parse {}: {}", file_path.display(), e))?;
            for (exchange_name, accounts) in data {
                let entry = combined.entry(exchange_name.clone()).or_default();
                for (name, secret) in accounts {
                    if entry.contains_key(&name) {
                        eprintln!(
                            "warning: duplicate key found across files: exchange={}, name={}, overriding with {}",
                            exchange_name,
                            name,
                            file_path.display()
                        );
                    }
                    // Later files override earlier files for duplicate keys.
                    entry.insert(name, secret);
                }
            }
        }
        self.storage.merge(combined)
    }
}
