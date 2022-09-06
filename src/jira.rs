use anyhow::Result;
use atlassian_jira_rest_types::v2::Comment;
use serde::{Deserialize, Serialize};

use crate::jira_types::IssueBean;

pub struct SearchGetParams {
    pub jql: String,
    pub start_at: Option<u64>,
    pub max_results: Option<u64>,
    pub validate_query: Option<bool>,
    pub fields: Option<Vec<String>>,
    pub expand: Option<Vec<String>>,
}

impl SearchGetParams {
    pub fn new(jql: &str) -> Self {
        Self {
            jql: jql.to_owned(),
            start_at: None,
            max_results: None,
            validate_query: None,
            fields: None,
            expand: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct JiraServer {
    pub base_url: url::Url,
    pub access: crate::authentication::Access,
}

impl JiraServer {
    pub async fn http_get(&self, path: &str, params: &[(&str, &str)]) -> Result<String> {
        let http_client = reqwest::ClientBuilder::new()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;
        let mut url = self.base_url.clone();
        url.set_path(path);
        url.query_pairs_mut().extend_pairs(params);

        let request = http_client
            .get(url.clone())
            .header(reqwest::header::ACCEPT, "application/json")
            .header(reqwest::header::CONTENT_TYPE, "application/json");
        let request = match &self.access {
            crate::authentication::Access::Token(secret) => request.header(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", secret.get()?.trim()),
            ),
        };

        let response = request.send().await?.error_for_status()?.text().await?;

        Ok(response)
    }

    pub async fn issue_beam(&self, issue: &str) -> Result<crate::jira_types::IssueBean> {
        let response = self
            .http_get(&format!("/rest/api/2/issue/{}", issue), &[])
            .await?;

        let json = serde_json::de::from_str::<atlassian_jira_rest_types::v2::IssueBean>(&response)?;
        crate::jira_types::IssueBean::of_json(json)
    }

    pub async fn search(
        &self,
        params: SearchGetParams,
    ) -> Result<atlassian_jira_rest_types::v2::SearchResults> {
        let mut query = vec![("jql", params.jql)];
        if let Some(v) = params.start_at {
            query.push(("startAt", format!("{}", v)))
        }
        if let Some(v) = params.max_results {
            query.push(("maxResult", format!("{}", v)))
        }
        if let Some(v) = params.validate_query {
            query.push(("validateQuery", format!("{}", v)))
        }
        if let Some(v) = params.fields {
            query.push(("fields", v.join(",")))
        }
        if let Some(v) = params.expand {
            query.push(("expand", v.join(",")))
        }

        let query: Vec<(&str, &str)> = query.iter().map(|(k, v)| (*k, v.as_str())).collect();
        let response = self
            .http_get("/rest/api/2/search", query.as_slice())
            .await?;

        let json =
            serde_json::de::from_str::<atlassian_jira_rest_types::v2::SearchResults>(&response)?;
        Ok(json)
    }
}

pub enum CommentPrinter {
    Email,
    Serde(crate::printer::SerdePrinter),
}

impl CommentPrinter {
    fn printer_email(&self, comment: &Comment) -> Result<String> {
        use std::fmt::Write;
        let mut output = String::new();
        writeln!(
            &mut output,
            "== Comment ======================================="
        )?;
        writeln!(
            &mut output,
            "From {:?} <{}>",
            comment
                .author
                .display_name
                .as_ref()
                .map(|v| v.as_str())
                .unwrap_or("No username"),
            comment
                .author
                .email_address
                .as_ref()
                .map(|v| v.as_str())
                .unwrap_or("no-email"),
        )?;
        writeln!(&mut output, "Date: {}", comment.created)?;
        if let Some(update_author) = &comment.update_author {
            writeln!(
                &mut output,
                "UpdatedBy: {:?} <{}>",
                update_author
                    .display_name
                    .as_ref()
                    .map(|v| v.as_str())
                    .unwrap_or("No username"),
                update_author
                    .email_address
                    .as_ref()
                    .map(|v| v.as_str())
                    .unwrap_or("no-email"),
            )?;
        }
        if let Some(updated) = &comment.updated {
            writeln!(&mut output, "UpdatedAt: {}", updated)?
        }
        writeln!(&mut output, "")?;
        writeln!(&mut output, "{}", comment.body)?;
        Ok(output)
    }

    pub fn data_to_string(&self, comment: &Comment) -> Result<String> {
        let r = match self {
            Self::Email => self.printer_email(comment)?,
            Self::Serde(printer) => printer.data_to_string(comment)?,
        };
        Ok(r)
    }
}

#[derive(Debug)]
pub enum IssuePrinter {
    Email,
    Serde(crate::printer::SerdePrinter),
}

impl IssuePrinter {
    fn printer_email(&self, issue: &IssueBean) -> Result<String> {
        use std::fmt::Write;
        let mut output = String::new();
        writeln!(
            &mut output,
            "From: {:?} <{}>",
            issue
                .fields
                .creator
                .display_name
                .as_ref()
                .map(|v| v.as_str())
                .unwrap_or("No username"),
            issue
                .fields
                .creator
                .email_address
                .as_ref()
                .map(|v| v.as_str())
                .unwrap_or("no-email"),
        )?;
        if let Some(assignee) = &issue.fields.assignee {
            writeln!(
                &mut output,
                "To: {:?} <{}>",
                assignee
                    .display_name
                    .as_ref()
                    .map(|v| v.as_str())
                    .unwrap_or("No username"),
                assignee
                    .email_address
                    .as_ref()
                    .map(|v| v.as_str())
                    .unwrap_or("no-email"),
            )?;
        }
        writeln!(&mut output, "Subject: {}", issue.fields.summary)?;
        writeln!(&mut output, "")?;
        writeln!(
            &mut output,
            "{}",
            issue.fields.description.clone().unwrap_or_default()
        )?;

        let comment_printer = CommentPrinter::Email;
        if let Some(comment_field) = &issue.fields.comment {
            for comment in &comment_field.comments {
                writeln!(
                    &mut output,
                    "\n{}",
                    comment_printer.data_to_string(comment)?
                )?
            }
        }
        Ok(output)
    }

    pub fn data_to_string(&self, issue: &IssueBean) -> Result<String> {
        let r = match self {
            Self::Email => self.printer_email(issue)?,
            Self::Serde(printer) => printer.data_to_string(issue)?,
        };
        Ok(r)
    }
}

impl std::str::FromStr for IssuePrinter {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "email" => Ok(Self::Email),
            _ => Ok(Self::Serde(crate::printer::SerdePrinter::from_str(s)?)),
        }
    }
}
