use crate::subcommands::io::{Arg, Data, Rtn};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

type RTN = Result<Rtn, String>;

pub struct FileDB {
    pub file_path: PathBuf,
}

impl FileDB {
    fn read_data(&self) -> Result<Data, String> {
        if !self.file_path.exists() {
            return Ok(HashMap::new());
        }
        fs::read_to_string(&self.file_path)
            .map_err(|e| format!("{:?}", e))
            .and_then(|content| {
                if content.trim().is_empty() {
                    Ok(HashMap::new())
                } else {
                    serde_json::from_str::<Data>(&content).map_err(|e| format!("{:?}", e))
                }
            })
    }

    fn write_data(&self, table: &Data) -> RTN {
        if let Some(parent) = self.file_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).map_err(|e| format!("{:?}", e))?;
            }
        }
        serde_json::to_string_pretty(table)
            .map_err(|e| format!("{:?}", e))
            .and_then(|json| fs::write(&self.file_path, json).map_err(|e| format!("{:?}", e)))
            .map(|_| Rtn::Empty)
    }

    fn after_data<F>(&self, cb: F) -> RTN
    where
        F: FnOnce(Data) -> RTN,
    {
        self.read_data().and_then(cb)
    }

    pub fn add(&self, arg: &Arg) -> RTN {
        self.after_data(|mut table| {
            arg.secret
                .to_owned()
                .ok_or(String::from("no secret supplied"))
                .and_then(|secret| match table.get_mut(&arg.exchange) {
                    Some(exchange_data) => match exchange_data.get(&arg.name) {
                        Some(_) => Err(String::from("account exists already")),
                        None => {
                            exchange_data.insert(arg.name.to_owned(), secret);
                            Ok(Rtn::Empty)
                        }
                    },
                    None => {
                        let mut exchange_data = HashMap::new();
                        exchange_data.insert(arg.name.to_owned(), secret);
                        table.insert(arg.exchange.to_owned(), exchange_data);
                        Ok(Rtn::Empty)
                    }
                })
                .and_then(|_| self.write_data(&table))
        })
    }

    pub fn update(&self, arg: &Arg) -> RTN {
        self.after_data(|mut table| {
            arg.secret
                .to_owned()
                .ok_or(String::from("no secret supplied"))
                .and_then(|secret| {
                    table
                        .get_mut(&arg.exchange)
                        .ok_or(String::from("no exchange found"))
                        .and_then(|exchange_data| {
                            if exchange_data.contains_key(&arg.name) {
                                exchange_data.insert(arg.name.to_owned(), secret);
                                Ok(Rtn::Empty)
                            } else {
                                Err(String::from("no account found"))
                            }
                        })
                        .and_then(|_| self.write_data(&table))
                })
        })
    }

    pub fn delete(&self, arg: &Arg) -> RTN {
        self.after_data(|mut table| {
            table
                .get_mut(&arg.exchange)
                .ok_or(String::from("no exchange found"))
                .and_then(|exchange_data| {
                    if exchange_data.contains_key(&arg.name) {
                        exchange_data.remove(&arg.name);
                        Ok(Rtn::Empty)
                    } else {
                        Err(String::from("no account found"))
                    }
                })
                .and_then(|_| self.write_data(&table))
        })
    }

    pub fn list(&self, exchange: Option<String>) -> RTN {
        self.after_data(|table| {
            let mut result = vec![];
            match &exchange {
                Some(exchange_name) => {
                    if let Some(exchange_data) = table.get(exchange_name) {
                        for (name, _) in exchange_data.iter() {
                            result.push(Rtn::Single {
                                exchange: exchange_name.to_owned(),
                                name: name.to_owned(),
                            })
                        }
                    }
                }
                None => {
                    for (exchange_name, exchange_data) in table.iter() {
                        for (name, _) in exchange_data.iter() {
                            result.push(Rtn::Single {
                                exchange: exchange_name.to_owned(),
                                name: name.to_owned(),
                            })
                        }
                    }
                }
            }
            Ok(Rtn::Multiple {
                data: Box::new(result),
            })
        })
    }

    pub fn get(&self, arg: &Arg) -> RTN {
        self.after_data(|table| {
            table
                .get(&arg.exchange)
                .ok_or(String::from("no exchange found"))
                .and_then(|exchange_data| {
                    exchange_data
                        .get(&arg.name)
                        .ok_or(String::from("no account found"))
                })
                .map(|secret| Rtn::Secret {
                    secret: secret.to_owned(),
                })
        })
    }
}
