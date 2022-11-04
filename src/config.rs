use std::collections::HashMap;

use anyhow::{format_err, Result};
use serde::{Deserialize, Serialize};

use crate::{confluence::ConfluenceServer, jira::JiraServer};

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
pub struct Report(#[serde(with = "serde_yaml::with::singleton_map")] pub crate::report::Report);

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub default_jira_instance: JiraServer,
    #[serde(default)]
    pub jira_instances: HashMap<String, JiraServer>,
    pub default_confluence_instance: ConfluenceServer,
    #[serde(default)]
    pub confluence_instances: HashMap<String, ConfluenceServer>,
    #[serde(default)]
    pub reports: HashMap<String, Report>,
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
