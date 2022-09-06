use std::str::FromStr;

use anyhow::Result;

#[derive(Debug)]
pub enum SerdePrinter {
    Yaml,
    Json,
}

impl SerdePrinter {
    pub fn data_to_string<DATA>(&self, v: &DATA) -> Result<String>
    where
        DATA: serde::Serialize,
    {
        let r = match self {
            Self::Yaml => serde_yaml::to_string(v)?,
            Self::Json => serde_json::to_string(v)?,
        };
        Ok(r)
    }
}

impl FromStr for SerdePrinter {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "json" => Ok(Self::Json),
            "yaml" => Ok(Self::Yaml),
            _ => Err("Unknown printer variant".to_owned()),
        }
    }
}
