use std::process::Command;

use anyhow::{bail, format_err, Result};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub enum Secret {
    String(String),
    Program(String),
}

impl Secret {
    fn of_command(command: &str) -> Result<String> {
        let output = if cfg!(target_os = "windows") {
            Command::new("cmd").args(["/C", command]).output()
        } else {
            Command::new("sh").args(["-c", command]).output()
        };
        let output =
            output.map_err(|err| format_err!("Failed to execute {:?}: {}", command, err))?;
        if !output.status.success() {
            bail!("Failed to execute secret command {:?}", command)
        }
        std::str::from_utf8(&output.stdout)
            .map_err(|err| {
                format_err!(
                    "Invalid UTF-8 sequence in command {:?} output: {}",
                    command,
                    err
                )
            })
            .map(|data| data.to_string())
    }

    pub fn get(&self) -> Result<String> {
        match self {
            Secret::String(v) => Ok(v.clone()),
            Secret::Program(command) => Self::of_command(command),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Hash, PartialEq, Eq, Debug)]
pub enum Access {
    #[serde(with = "serde_yaml::with::singleton_map")]
    Token(Secret),
    #[serde(with = "serde_yaml::with::singleton_map")]
    JSessionID(Secret),
}
