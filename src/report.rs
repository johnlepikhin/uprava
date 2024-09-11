use std::sync::Arc;

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub enum ReportResult {
    ConfluenceRoadmap(crate::report_confluence_roadmap::ConfluenceRoadmap),
}

#[derive(Clone, PartialEq, Eq)]
pub enum ReportIssueType {
    ReportMember,
    ExternalDependency,
    Epic,
}

#[derive(Clone)]
pub struct ReportIssue {
    pub jira: crate::jira::JiraServer,
    pub issue: crate::jira_types::IssueBean,
    pub custom_fields: crate::jira::IssueCustomFields,
    pub entity_type: ReportIssueType,
}

impl ReportIssue {
    pub fn of_issuebean(
        jira: &crate::jira::JiraServer,
        issue: &crate::jira_types::IssueBean,
        entity_type: ReportIssueType,
    ) -> Result<Self> {
        let custom_fields = crate::jira::IssueCustomFields::of_issue(jira, issue)?;
        Ok(Self {
            jira: jira.clone(),
            issue: issue.clone(),
            custom_fields,
            entity_type,
        })
    }

    pub fn url(&self) -> url::Url {
        let mut url = self.jira.base_url.clone();
        url.set_path(&format!("browse/{}", self.issue.key));
        url
    }

    pub fn confluence_wiki_url(&self, newlines: bool) -> String {
        let url = self.url();
        format!(
            "[{}|{}] {} {}",
            self.issue.key,
            url,
            if newlines { "\\\\" } else { "" },
            crate::confluence::wiki_escape(
                &self
                    .issue
                    .fields
                    .status
                    .as_ref()
                    .and_then(|v| v.name.clone())
                    .unwrap_or_default()
            )
        )
    }

    pub fn confluence_wiki_epic_url(&self) -> String {
        let url = self.url();
        format!(
            "[{}|{}]",
            self.custom_fields.epic_name.as_deref().unwrap_or_default(),
            url,
        )
    }

    pub fn confluence_wiki_schedule(&self) -> String {
        let duration = self.custom_fields.plan();
        let planned_end = self
            .custom_fields
            .planned_end
            .unwrap_or_else(|| chrono::Utc::now().date() + chrono::Duration::days(100000));
        let planned_start = self
            .custom_fields
            .planned_start
            .unwrap_or_else(|| chrono::Utc::now().date() + chrono::Duration::days(100000));
        if planned_end - chrono::Duration::days(3) < chrono::Utc::now().date() {
            format!("{{color:red}}{}{{color}}", duration)
        } else if planned_start - chrono::Duration::days(3) < chrono::Utc::now().date() {
            format!("{{color:green}}{}{{color}}", duration)
        } else {
            duration
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ForeignRelationSubject {
    pub jira: crate::jira::JiraServer,
    pub issue: String,
    pub kind: String,
}

#[derive(Hash, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub enum RelationKind {
    Dependance,
    Block,
    Mention,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ForeignRelation {
    pub from: ForeignRelationSubject,
    pub to: ForeignRelationSubject,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct QuerySet(Vec<crate::config::JiraQuery>);

impl QuerySet {
    pub async fn get_issues(&self, config: Arc<crate::config::Config>) -> Result<Vec<ReportIssue>> {
        let mut issues_list = Vec::new();
        let mut join_set = tokio::task::JoinSet::new();
        for query in &self.0 {
            let query_clone = query.clone();
            let config = config.clone();
            let _abort_handle = join_set.spawn(async move {
                let mut query_string = query_clone.query.replace('\n', " ").trim().to_string();
                for (subst_key, subst_value) in &config.substitutions {
                    query_string = query_string.replace(&format!("%{subst_key}%"), &subst_value);
                }
                slog_scope::info!("Querying JIRA: {}", query_string);
                let handler = query_clone
                    .jira
                    .search_all(&crate::jira::SearchGetParams::new(&query_string))
                    .await;
                (handler, query_clone)
            });
        }
        while let Some(pair) = join_set.join_next().await {
            let (result, query) = pair?;
            for issue in result? {
                let issue = crate::jira_types::IssueBean::of_json(issue)?;
                issues_list.push(ReportIssue::of_issuebean(
                    &query.jira,
                    &issue,
                    ReportIssueType::ReportMember,
                )?)
            }
        }
        Ok(issues_list)
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub enum Report {
    #[serde(with = "serde_yaml::with::singleton_map")]
    ConfluenceRoadmap(crate::report_confluence_roadmap::ConfluenceRoadmap),
    #[serde(with = "serde_yaml::with::singleton_map")]
    Worklog(crate::report_worklog::Worklog),
}
