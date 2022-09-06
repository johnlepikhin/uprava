use std::collections::HashMap;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub enum ReportResult {
    Roadmap(crate::report_roadmap::Roadmap),
}

pub struct ReportIssue {
    pub jira: crate::jira::JiraServer,
    pub issue: crate::jira_types::IssueBean,
    pub custom_fields: HashMap<String, serde_json::Value>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Report {
    pub queries: Vec<crate::config::JiraQuery>,
    pub result: ReportResult,
}

impl Report {
    pub async fn get_issues(&self) -> Result<Vec<ReportIssue>> {
        let mut issues_list = Vec::new();
        for query in &self.queries {
            let result = query
                .jira
                .search(crate::jira::SearchGetParams::new(&query.query))
                .await?;
            let issues = match result.issues {
                None => continue,
                Some(v) => v,
            };
            for issue in issues {
                let issue = crate::jira_types::IssueBean::of_json(issue)?;
                let mut custom_fields = HashMap::new();
                for (src_name, custom_field) in &query.custom_fields {
                    let src_value = match issue.fields.custom_fields.get(src_name) {
                        None => bail!(
                            "Report requires custom field {:?} but it doesn't exist in issue {:?}",
                            src_name,
                            issue.key
                        ),
                        Some(v) => v,
                    };
                    let _ = custom_fields.insert(custom_field.name.clone(), src_value.clone());
                }
                issues_list.push(ReportIssue {
                    jira: query.jira.clone(),
                    issue,
                    custom_fields,
                })
            }
        }
        Ok(issues_list)
    }

    pub async fn make(&self) -> Result<()> {
        let issues_list = self.get_issues().await?;
        match &self.result {
            ReportResult::Roadmap(v) => v.make(&issues_list).await?,
        }
        Ok(())
    }
}
