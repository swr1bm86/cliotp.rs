use crate::subcommands::io::{Arg, Data, Rtn, Storage};
use std::collections::HashMap;

pub use r2d2_redis::r2d2::PooledConnection;
pub use r2d2_redis::redis::{Commands, Connection, RedisResult};
pub use r2d2_redis::{r2d2, RedisConnectionManager};

type RTN = Result<Rtn, String>;

pub struct DB<'a> {
    pub db_name: &'a str,
    pub pool: &'a r2d2::Pool<RedisConnectionManager>,
}

impl<'a> DB<'a> {
    fn after_conn<F>(&self, cb: F) -> RTN
    where
        F: FnOnce(PooledConnection<RedisConnectionManager>) -> RTN,
    {
        self.pool.get().map_err(|e| format!("{:?}", e)).and_then(cb)
    }

    fn after_data<F>(&self, cb: F) -> RTN
    where
        F: FnMut(Data) -> RTN,
    {
        self.after_conn(|mut conn| {
            conn.get(self.db_name)
                .map_or(Ok(HashMap::new()), |data: String| {
                    serde_json::from_str::<Data>(&data).map_err(|e| format!("{:?}", e))
                })
                .and_then(cb)
        })
    }

    fn update_table(&self, table: Data) -> RTN {
        serde_json::to_string(&table)
            .map_err(|e| format!("{:?}", e))
            .and_then(|t| {
                self.after_conn(|mut conn| {
                    let _: RedisResult<String> = conn.set(self.db_name, t);
                    Ok(Rtn::Empty)
                })
            })
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
                .and_then(|_| self.update_table(table))
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
                        .and_then(|_| self.update_table(table))
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
                .and_then(|_| self.update_table(table))
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

impl<'a> Storage for DB<'a> {
    fn add(&self, arg: &Arg) -> Result<Rtn, String> {
        self.add(arg)
    }

    fn update(&self, arg: &Arg) -> Result<Rtn, String> {
        self.update(arg)
    }

    fn delete(&self, arg: &Arg) -> Result<Rtn, String> {
        self.delete(arg)
    }

    fn list(&self, exchange: Option<String>) -> Result<Rtn, String> {
        self.list(exchange)
    }

    fn get(&self, arg: &Arg) -> Result<Rtn, String> {
        self.get(arg)
    }
}
