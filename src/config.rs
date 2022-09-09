use std::collections::HashMap;

use anyhow::{format_err, Result};
use serde::{Deserialize, Serialize};

use crate::jira::JiraServer;

#[derive(Serialize, Deserialize, Clone)]
pub struct JiraQueryCustomField {
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct JiraQuery {
    pub jira: JiraServer,
    pub query: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub default_jira_instance: JiraServer,
    pub extra_jira_instances: HashMap<String, JiraServer>,
    pub reports: HashMap<String, crate::report::Report>,
}

impl Config {
    pub fn read(file: &str) -> Result<Self> {
        let config = std::fs::read_to_string(file)
            .map_err(|err| format_err!("Failed to load config file {:?}: {}", file, err))?;
        let config: Self = serde_yaml::from_str(&config)
            .map_err(|err| format_err!("Failed to parse config file {:?}: {}", file, err))?;
        Ok(config)
    }
}
