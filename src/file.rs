use crate::subcommands::io::{Arg, Data, Rtn, Storage};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

type SafeRtn = Result<Rtn, String>;

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

    fn write_data(&self, table: &Data) -> SafeRtn {
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

    fn after_data<F>(&self, cb: F) -> SafeRtn
    where
        F: FnOnce(Data) -> SafeRtn,
    {
        self.read_data().and_then(cb)
    }

    pub fn add(&self, arg: &Arg) -> SafeRtn {
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

    pub fn update(&self, arg: &Arg) -> SafeRtn {
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

    pub fn delete(&self, arg: &Arg) -> SafeRtn {
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

    pub fn list(&self, exchange: Option<String>) -> SafeRtn {
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
            Ok(Rtn::Multiple { data: result })
        })
    }

    pub fn get(&self, arg: &Arg) -> SafeRtn {
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

impl Storage for FileDB {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tmp_db(name: &str) -> FileDB {
        let dir = std::env::temp_dir().join("cliotp_test");
        let _ = fs::create_dir_all(&dir);
        let file_path = dir.join(format!("{}.json", name));
        let _ = fs::remove_file(&file_path);
        FileDB { file_path }
    }

    fn make_arg(exchange: &str, name: &str, secret: Option<&str>) -> Arg {
        Arg {
            exchange: exchange.to_owned(),
            name: name.to_owned(),
            secret: secret.map(|s| s.to_owned()),
        }
    }

    // ── Add ──

    #[test]
    fn test_add_new_account() {
        let db = tmp_db("add_new");
        let arg = make_arg("test_exchange", "alice", Some("SECRET1"));
        let result = db.add(&arg);
        assert!(result.is_ok());

        // verify persisted
        let data = db.read_data().unwrap();
        assert_eq!(data["test_exchange"]["alice"], "SECRET1");
    }

    #[test]
    fn test_add_second_account_same_exchange() {
        let db = tmp_db("add_second");
        db.add(&make_arg("test_exchange", "alice", Some("S1")))
            .unwrap();
        db.add(&make_arg("test_exchange", "bob", Some("S2")))
            .unwrap();

        let data = db.read_data().unwrap();
        assert_eq!(data["test_exchange"].len(), 2);
        assert_eq!(data["test_exchange"]["alice"], "S1");
        assert_eq!(data["test_exchange"]["bob"], "S2");
    }

    #[test]
    fn test_add_different_exchanges() {
        let db = tmp_db("add_diff_ex");
        db.add(&make_arg("test_exchange", "alice", Some("S1")))
            .unwrap();
        db.add(&make_arg("test_exchange_2", "bob", Some("S2")))
            .unwrap();

        let data = db.read_data().unwrap();
        assert_eq!(data.len(), 2);
        assert_eq!(data["test_exchange"]["alice"], "S1");
        assert_eq!(data["test_exchange_2"]["bob"], "S2");
    }

    #[test]
    fn test_add_duplicate_account_fails() {
        let db = tmp_db("add_dup");
        db.add(&make_arg("test_exchange", "alice", Some("S1")))
            .unwrap();
        let result = db.add(&make_arg("test_exchange", "alice", Some("S2")));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "account exists already");
    }

    #[test]
    fn test_add_no_secret_fails() {
        let db = tmp_db("add_no_secret");
        let result = db.add(&make_arg("test_exchange", "alice", None));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "no secret supplied");
    }

    // ── Update ──

    #[test]
    fn test_update_existing_account() {
        let db = tmp_db("update_ok");
        db.add(&make_arg("test_exchange", "alice", Some("OLD")))
            .unwrap();
        let result = db.update(&make_arg("test_exchange", "alice", Some("NEW")));
        assert!(result.is_ok());

        let data = db.read_data().unwrap();
        assert_eq!(data["test_exchange"]["alice"], "NEW");
    }

    #[test]
    fn test_update_no_exchange_fails() {
        let db = tmp_db("update_no_ex");
        let result = db.update(&make_arg("test_exchange", "alice", Some("NEW")));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "no exchange found");
    }

    #[test]
    fn test_update_no_account_fails() {
        let db = tmp_db("update_no_acc");
        db.add(&make_arg("test_exchange", "alice", Some("S1")))
            .unwrap();
        let result = db.update(&make_arg("test_exchange", "bob", Some("NEW")));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "no account found");
    }

    #[test]
    fn test_update_no_secret_fails() {
        let db = tmp_db("update_no_secret");
        db.add(&make_arg("test_exchange", "alice", Some("S1")))
            .unwrap();
        let result = db.update(&make_arg("test_exchange", "alice", None));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "no secret supplied");
    }

    // ── Delete ──

    #[test]
    fn test_delete_existing_account() {
        let db = tmp_db("delete_ok");
        db.add(&make_arg("test_exchange", "alice", Some("S1")))
            .unwrap();
        let result = db.delete(&make_arg("test_exchange", "alice", None));
        assert!(result.is_ok());

        let data = db.read_data().unwrap();
        assert!(!data["test_exchange"].contains_key("alice"));
    }

    #[test]
    fn test_delete_no_exchange_fails() {
        let db = tmp_db("delete_no_ex");
        let result = db.delete(&make_arg("test_exchange", "alice", None));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "no exchange found");
    }

    #[test]
    fn test_delete_no_account_fails() {
        let db = tmp_db("delete_no_acc");
        db.add(&make_arg("test_exchange", "alice", Some("S1")))
            .unwrap();
        let result = db.delete(&make_arg("test_exchange", "bob", None));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "no account found");
    }

    #[test]
    fn test_delete_does_not_affect_others() {
        let db = tmp_db("delete_others");
        db.add(&make_arg("test_exchange", "alice", Some("S1")))
            .unwrap();
        db.add(&make_arg("test_exchange", "bob", Some("S2")))
            .unwrap();
        db.delete(&make_arg("test_exchange", "alice", None))
            .unwrap();

        let data = db.read_data().unwrap();
        assert_eq!(data["test_exchange"].len(), 1);
        assert_eq!(data["test_exchange"]["bob"], "S2");
    }

    // ── List ──

    #[test]
    fn test_list_all_empty() {
        let db = tmp_db("list_empty");
        let result = db.list(None).unwrap();
        match result {
            Rtn::Multiple { data } => assert!(data.is_empty()),
            _ => panic!("expected Rtn::Multiple"),
        }
    }

    #[test]
    fn test_list_all() {
        let db = tmp_db("list_all");
        db.add(&make_arg("test_exchange", "alice", Some("S1")))
            .unwrap();
        db.add(&make_arg("test_exchange_2", "bob", Some("S2")))
            .unwrap();

        let result = db.list(None).unwrap();
        match result {
            Rtn::Multiple { data } => assert_eq!(data.len(), 2),
            _ => panic!("expected Rtn::Multiple"),
        }
    }

    #[test]
    fn test_list_by_exchange() {
        let db = tmp_db("list_by_ex");
        db.add(&make_arg("test_exchange", "alice", Some("S1")))
            .unwrap();
        db.add(&make_arg("test_exchange", "bob", Some("S2")))
            .unwrap();
        db.add(&make_arg("test_exchange_2", "carol", Some("S3")))
            .unwrap();

        let result = db.list(Some("test_exchange".to_owned())).unwrap();
        match result {
            Rtn::Multiple { data } => {
                assert_eq!(data.len(), 2);
                for item in &data {
                    match item {
                        Rtn::Single { exchange, .. } => assert_eq!(exchange, "test_exchange"),
                        _ => panic!("expected Rtn::Single inside Multiple"),
                    }
                }
            }
            _ => panic!("expected Rtn::Multiple"),
        }
    }

    #[test]
    fn test_list_nonexistent_exchange_returns_empty() {
        let db = tmp_db("list_no_ex");
        db.add(&make_arg("test_exchange", "alice", Some("S1")))
            .unwrap();

        let result = db
            .list(Some("test_exchange_nonexistent".to_owned()))
            .unwrap();
        match result {
            Rtn::Multiple { data } => assert!(data.is_empty()),
            _ => panic!("expected Rtn::Multiple"),
        }
    }

    // ── Get (used by Now subcommand) ──

    #[test]
    fn test_get_existing_account() {
        let db = tmp_db("get_ok");
        db.add(&make_arg("test_exchange", "alice", Some("MYSECRET")))
            .unwrap();

        let result = db.get(&make_arg("test_exchange", "alice", None)).unwrap();
        match result {
            Rtn::Secret { secret } => assert_eq!(secret, "MYSECRET"),
            _ => panic!("expected Rtn::Secret"),
        }
    }

    #[test]
    fn test_get_no_exchange_fails() {
        let db = tmp_db("get_no_ex");
        let result = db.get(&make_arg("test_exchange", "alice", None));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "no exchange found");
    }

    #[test]
    fn test_get_no_account_fails() {
        let db = tmp_db("get_no_acc");
        db.add(&make_arg("test_exchange", "alice", Some("S1")))
            .unwrap();
        let result = db.get(&make_arg("test_exchange", "bob", None));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "no account found");
    }

    // ── Storage trait dispatch ──

    #[test]
    fn test_storage_trait_add_and_get() {
        let db = tmp_db("trait_add_get");
        let storage: &dyn Storage = &db;

        storage
            .add(&make_arg("test_exchange", "alice", Some("TRAITKEY")))
            .unwrap();
        let result = storage
            .get(&make_arg("test_exchange", "alice", None))
            .unwrap();
        match result {
            Rtn::Secret { secret } => assert_eq!(secret, "TRAITKEY"),
            _ => panic!("expected Rtn::Secret"),
        }
    }

    #[test]
    fn test_storage_trait_update() {
        let db = tmp_db("trait_update");
        let storage: &dyn Storage = &db;

        storage
            .add(&make_arg("test_exchange", "alice", Some("OLD")))
            .unwrap();
        storage
            .update(&make_arg("test_exchange", "alice", Some("NEW")))
            .unwrap();

        let result = storage
            .get(&make_arg("test_exchange", "alice", None))
            .unwrap();
        match result {
            Rtn::Secret { secret } => assert_eq!(secret, "NEW"),
            _ => panic!("expected Rtn::Secret"),
        }
    }

    #[test]
    fn test_storage_trait_delete() {
        let db = tmp_db("trait_delete");
        let storage: &dyn Storage = &db;

        storage
            .add(&make_arg("test_exchange", "alice", Some("S1")))
            .unwrap();
        storage
            .delete(&make_arg("test_exchange", "alice", None))
            .unwrap();

        let result = storage.get(&make_arg("test_exchange", "alice", None));
        assert!(result.is_err());
    }

    #[test]
    fn test_storage_trait_list() {
        let db = tmp_db("trait_list");
        let storage: &dyn Storage = &db;

        storage
            .add(&make_arg("test_exchange", "alice", Some("S1")))
            .unwrap();
        let result = storage.list(None).unwrap();
        match result {
            Rtn::Multiple { data } => assert_eq!(data.len(), 1),
            _ => panic!("expected Rtn::Multiple"),
        }
    }

    // ── File persistence ──

    #[test]
    fn test_data_persists_across_instances() {
        let dir = std::env::temp_dir().join("cliotp_test");
        let _ = fs::create_dir_all(&dir);
        let file_path = dir.join("persist.json");
        let _ = fs::remove_file(&file_path);

        let db1 = FileDB {
            file_path: file_path.clone(),
        };
        db1.add(&make_arg("test_exchange", "alice", Some("PERSIST")))
            .unwrap();

        // new instance reading same file
        let db2 = FileDB { file_path };
        let result = db2.get(&make_arg("test_exchange", "alice", None)).unwrap();
        match result {
            Rtn::Secret { secret } => assert_eq!(secret, "PERSIST"),
            _ => panic!("expected Rtn::Secret"),
        }
    }

    #[test]
    fn test_read_empty_file() {
        let dir = std::env::temp_dir().join("cliotp_test");
        let _ = fs::create_dir_all(&dir);
        let file_path = dir.join("empty.json");
        fs::write(&file_path, "").unwrap();

        let db = FileDB { file_path };
        let data = db.read_data().unwrap();
        assert!(data.is_empty());
    }

    #[test]
    fn test_read_nonexistent_file() {
        let file_path = std::env::temp_dir()
            .join("cliotp_test")
            .join("does_not_exist.json");
        let _ = fs::remove_file(&file_path);

        let db = FileDB { file_path };
        let data = db.read_data().unwrap();
        assert!(data.is_empty());
    }

    #[test]
    fn test_creates_parent_dirs() {
        let dir = std::env::temp_dir()
            .join("cliotp_test")
            .join("nested")
            .join("deep");
        let _ = fs::remove_dir_all(&dir);
        let file_path = dir.join("otp.json");

        let db = FileDB { file_path };
        db.add(&make_arg("test_exchange", "alice", Some("S1")))
            .unwrap();

        let data = db.read_data().unwrap();
        assert_eq!(data["test_exchange"]["alice"], "S1");
    }
}
