use anyhow::Result;
use atlassian_jira_rest_types::v2::Comment;
use chrono::Datelike;
use serde::{Deserialize, Serialize};

use crate::jira_types::IssueBean;

#[derive(Clone, Debug)]
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

#[derive(Serialize, Deserialize, Clone, Hash, PartialEq, Eq, Debug)]
pub struct CustomField {
    pub name: String,
}

impl CustomField {
    pub fn of_issue(&self, issue: &crate::jira_types::IssueBean) -> Result<Option<String>> {
        let r = match self.name.as_str() {
            "summary" => Some(issue.fields.summary.clone()),
            "description" => issue.fields.description.clone(),
            "key" => Some(issue.key.clone()),
            "id" => Some(issue.id.clone()),
            "issuetype.name" => issue
                .fields
                .issuetype
                .as_ref()
                .map(|v| v.name.clone())
                .flatten(),
            v => match issue.fields.custom_fields.get(v) {
                None => None,
                Some(v) => serde_json::value::from_value(v.clone())?,
            },
        };

        Ok(r)
    }

    pub fn date_of_issue(
        &self,
        issue: &crate::jira_types::IssueBean,
    ) -> Result<Option<chrono::Date<chrono::Utc>>> {
        let s: Option<String> = self.of_issue(issue)?;
        match s {
            None => Ok(None),
            Some(v) => {
                let v = format!("\"{}T00:00:00.000000Z\"", v);
                slog_scope::debug!("Parsing DateTime from string: {}", v);
                let r: chrono::DateTime<chrono::Utc> = serde_json::from_str(&v)?;
                Ok(Some(r.date()))
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Hash, PartialEq, Eq, Debug)]
pub struct IssueCustomFieldsConfig {
    pub reason: CustomField,
    pub epic_link: CustomField,
    pub epic_name: CustomField,
    pub planned_start: CustomField,
    pub planned_end: CustomField,
}

#[derive(Clone)]
pub struct IssueCustomFields {
    pub reason: Option<String>,
    pub epic_link: Option<String>,
    pub epic_name: Option<String>,
    pub planned_start: Option<chrono::Date<chrono::Utc>>,
    pub planned_end: Option<chrono::Date<chrono::Utc>>,
}

impl IssueCustomFields {
    pub fn of_issue(jira: &JiraServer, issue: &crate::jira_types::IssueBean) -> Result<Self> {
        Ok(Self {
            reason: jira.custom_fields.reason.of_issue(issue)?,
            epic_link: jira.custom_fields.epic_link.of_issue(issue)?,
            epic_name: jira.custom_fields.epic_name.of_issue(issue)?,
            planned_start: jira.custom_fields.planned_start.date_of_issue(issue)?,
            planned_end: jira.custom_fields.planned_end.date_of_issue(issue)?,
        })
    }

    fn format_date(date: &Option<chrono::Date<chrono::Utc>>) -> String {
        match date {
            None => "?".to_owned(),
            Some(v) => format!("{}-{:02}-{:02}", v.year(), v.month(), v.day()),
        }
    }

    pub fn plan(&self) -> String {
        if self.planned_start.is_some() || self.planned_end.is_some() {
            format!(
                "{} - {}",
                Self::format_date(&self.planned_start),
                Self::format_date(&self.planned_end)
            )
        } else {
            "".to_owned()
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Hash, PartialEq, Eq, Debug)]
pub struct JiraServer {
    pub base_url: url::Url,
    #[serde(with = "serde_yaml::with::singleton_map")]
    pub access: crate::authentication::Access,
    pub custom_fields: IssueCustomFieldsConfig,
    #[serde(default)]
    pub relations_map: Vec<(String, String)>,
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
            crate::authentication::Access::JSessionID(secret) => request.header(
                reqwest::header::COOKIE,
                format!("JSESSIONID={}", secret.get()?.trim()),
            ),
        };

        let response = request.send().await?.error_for_status()?.text().await?;

        Ok(response)
    }

    pub async fn issue_bean(&self, issue: &str) -> Result<crate::jira_types::IssueBean> {
        slog_scope::info!("Getting issue from {:?}: {:?}", self.base_url, issue);

        let response = self
            .http_get(&format!("/rest/api/2/issue/{}", issue), &[])
            .await?;

        slog_scope::trace!("Got from {:?}: {:?}", self.base_url, response);

        let json = serde_json::de::from_str::<atlassian_jira_rest_types::v2::IssueBean>(&response)?;
        crate::jira_types::IssueBean::of_json(json)
    }

    pub async fn search(
        &self,
        params: &SearchGetParams,
    ) -> Result<atlassian_jira_rest_types::v2::SearchResults> {
        let mut query = vec![("jql", params.jql.clone())];
        if let Some(v) = params.start_at {
            query.push(("startAt", format!("{}", v)))
        }
        if let Some(v) = params.max_results {
            query.push(("maxResult", format!("{}", v)))
        }
        if let Some(v) = params.validate_query {
            query.push(("validateQuery", format!("{}", v)))
        }
        if let Some(v) = &params.fields {
            query.push(("fields", v.join(",")))
        }
        if let Some(v) = &params.expand {
            query.push(("expand", v.join(",")))
        }

        let query: Vec<(&str, &str)> = query.iter().map(|(k, v)| (*k, v.as_str())).collect();

        slog_scope::info!("Searching on {:?}: {:?}", self.base_url, params);

        let response = self
            .http_get("/rest/api/2/search", query.as_slice())
            .await?;

        let json =
            serde_json::de::from_str::<atlassian_jira_rest_types::v2::SearchResults>(&response)?;
        Ok(json)
    }

    pub async fn search_all(
        &self,
        params: &SearchGetParams,
    ) -> Result<Vec<atlassian_jira_rest_types::v2::IssueBean>> {
        let mut result_list = Vec::new();
        loop {
            let r = self
                .search(&SearchGetParams {
                    start_at: Some(result_list.len() as u64),
                    max_results: Some(1000),
                    ..params.clone()
                })
                .await?;
            match r.issues {
                None => break,
                Some(issues) => {
                    if issues.is_empty() {
                        break;
                    }
                    let issues_count = issues.len();
                    result_list.extend(issues);
                    if let Some(per_page) = r.max_results {
                        if issues_count < per_page as usize {
                            break;
                        }
                    }
                }
            }
        }
        Ok(result_list)
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
                .as_deref()
                .unwrap_or("No username"),
            comment
                .author
                .email_address
                .as_deref()
                .unwrap_or("no-email"),
        )?;
        writeln!(&mut output, "Date: {}", comment.created)?;
        if let Some(update_author) = &comment.update_author {
            writeln!(
                &mut output,
                "UpdatedBy: {:?} <{}>",
                update_author
                    .display_name
                    .as_deref()
                    .unwrap_or("No username"),
                update_author.email_address.as_deref().unwrap_or("no-email"),
            )?;
        }
        if let Some(updated) = &comment.updated {
            writeln!(&mut output, "UpdatedAt: {}", updated)?
        }
        writeln!(&mut output)?;
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

#[derive(Debug, Clone)]
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
                .as_deref()
                .unwrap_or("No username"),
            issue
                .fields
                .creator
                .email_address
                .as_deref()
                .unwrap_or("no-email"),
        )?;
        if let Some(assignee) = &issue.fields.assignee {
            writeln!(
                &mut output,
                "To: {:?} <{}>",
                assignee.display_name.as_deref().unwrap_or("No username"),
                assignee.email_address.as_deref().unwrap_or("no-email"),
            )?;
        }
        writeln!(&mut output, "Subject: {}", issue.fields.summary)?;
        writeln!(&mut output)?;
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
