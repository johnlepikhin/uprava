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

fn default_dependencies_deepness() -> usize {
    1
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Report {
    pub queries: Vec<crate::config::JiraQuery>,
    pub results: Vec<ReportResult>,
    #[serde(default)]
    pub foreign_relations: Vec<ForeignRelation>,
    #[serde(default = "default_dependencies_deepness")]
    pub dependencies_deepness: usize,
}

impl Report {
    pub async fn get_issues(&self) -> Result<Vec<ReportIssue>> {
        let mut issues_list = Vec::new();
        let mut join_set = tokio::task::JoinSet::new();
        for query in &self.queries {
            let query_clone = query.clone();
            let _abort_handle = join_set.spawn(async move {
                let handler = query_clone
                    .jira
                    .search_all(&crate::jira::SearchGetParams::new(
                        query_clone.query.replace('\n', " ").trim(),
                    ))
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

    pub async fn make(&self) -> Result<()> {
        let issues_list = self.get_issues().await?;
        let data = crate::report_data::ReportData::of_slice(
            &self.foreign_relations,
            &issues_list,
            self.dependencies_deepness,
        )
        .await?;
        for result in &self.results {
            match result {
                ReportResult::ConfluenceRoadmap(v) => v.make(&data).await?,
            }
        }
        Ok(())
    }
}
