use std::collections::HashMap;
use std::fmt;

#[derive(Eq, PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct Arg {
    pub exchange: String,
    pub name: String,
    pub secret: Option<String>,
}

type Exchange = String;
type Name = String;
type Secret = String;
pub type Data = HashMap<Exchange, HashMap<Name, Secret>>;

pub trait Storage {
    fn add(&self, arg: &Arg) -> Result<Rtn, String>;
    fn update(&self, arg: &Arg) -> Result<Rtn, String>;
    fn delete(&self, arg: &Arg) -> Result<Rtn, String>;
    fn list(&self, exchange: Option<String>) -> Result<Rtn, String>;
    fn get(&self, arg: &Arg) -> Result<Rtn, String>;
    fn merge(&self, incoming: Data) -> Result<Rtn, String>;
}

/// Parse a JSON string into Data, supporting both plain JSON and
/// Redis-exported escaped string format (e.g. `"{\"k\":{\"n\":\"s\"}}"`)
pub fn parse_data(content: &str) -> Result<Data, String> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Ok(HashMap::new());
    }

    // Try plain JSON first
    if let Ok(data) = serde_json::from_str::<Data>(trimmed) {
        return Ok(data);
    }

    // Try Redis escaped string: outer layer is a JSON string containing escaped JSON
    if let Ok(inner) = serde_json::from_str::<String>(trimmed) {
        return serde_json::from_str::<Data>(&inner).map_err(|e| format!("{:?}", e));
    }

    Err(String::from(
        "invalid format: expected plain JSON or Redis escaped JSON string",
    ))
}

#[derive(Debug)]
pub enum Rtn {
    Empty,
    Code { code: String },
    Secret { secret: String },
    Single { exchange: String, name: String },
    Multiple { data: Vec<Rtn> },
    MergeResult { added: u32, skipped: u32 },
}

impl fmt::Display for Rtn {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            Rtn::Empty => write!(f, ""),
            Rtn::Code { code } => write!(f, "{}", code),
            Rtn::Secret { .. } => write!(f, "$$$"),
            Rtn::Single { exchange, name } => write!(f, "{} -> {}", exchange, name),
            Rtn::Multiple { data } => {
                for rtn in data.iter() {
                    rtn.fmt(f).unwrap();
                    writeln!(f).unwrap();
                }
                Ok(())
            }
            Rtn::MergeResult { added, skipped } => {
                write!(f, "merged: {} added, {} skipped", added, skipped)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_data_plain_json() {
        let json = r#"{"test_exchange":{"alice":"SECRET1","bob":"SECRET2"}}"#;
        let data = parse_data(json).unwrap();
        assert_eq!(data["test_exchange"]["alice"], "SECRET1");
        assert_eq!(data["test_exchange"]["bob"], "SECRET2");
    }

    #[test]
    fn test_parse_data_pretty_json() {
        let json = r#"{
  "test_exchange": {
    "alice": "SECRET1"
  }
}"#;
        let data = parse_data(json).unwrap();
        assert_eq!(data["test_exchange"]["alice"], "SECRET1");
    }

    #[test]
    fn test_parse_data_redis_escaped() {
        // Redis dumps the value as a JSON string with inner escaped JSON
        let redis_json = r##""{\"test_exchange\":{\"alice\":\"SECRET1\"}}""##;
        let data = parse_data(redis_json).unwrap();
        assert_eq!(data["test_exchange"]["alice"], "SECRET1");
    }

    #[test]
    fn test_parse_data_redis_escaped_multiple_exchanges() {
        let redis_json =
            r##""{\"test_exchange\":{\"alice\":\"S1\"},\"test_exchange_2\":{\"bob\":\"S2\"}}""##;
        let data = parse_data(redis_json).unwrap();
        assert_eq!(data["test_exchange"]["alice"], "S1");
        assert_eq!(data["test_exchange_2"]["bob"], "S2");
    }

    #[test]
    fn test_parse_data_empty_string() {
        let data = parse_data("").unwrap();
        assert!(data.is_empty());
    }

    #[test]
    fn test_parse_data_whitespace_only() {
        let data = parse_data("   \n  ").unwrap();
        assert!(data.is_empty());
    }

    #[test]
    fn test_parse_data_invalid_json() {
        let result = parse_data("not json at all");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_data_valid_string_but_invalid_inner_json() {
        // A valid JSON string, but inner content is not valid Data
        let result = parse_data(r#""just a plain string""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_data_empty_object() {
        let data = parse_data("{}").unwrap();
        assert!(data.is_empty());
    }

    #[test]
    fn test_parse_data_redis_escaped_empty_object() {
        let data = parse_data(r#""{}""#).unwrap();
        assert!(data.is_empty());
    }
}
