use google_authenticator::GA_AUTH;

use super::io::{Arg, Rtn, Storage};

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
