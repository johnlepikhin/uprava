use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::fmt::Write;

use crate::report::ReportIssue;

#[derive(Serialize, Deserialize, Clone)]
pub struct Member {
    name: String,
    query_set: crate::report::QuerySet,
    #[serde(default)]
    description: Option<String>,
}

pub struct MemberResult {
    member: Member,
    issues: Vec<ReportIssue>,
}

impl MemberResult {
    fn get_task(&self, issue: &crate::report::ReportIssue) -> String {
        let mut col1 = format!(
            " *{}*",
            crate::confluence::wiki_escape(&issue.issue.fields.summary)
        );
        if let Some(reason) = &issue.custom_fields.reason {
            col1 = format!(
                "{}\\\\ \\\\{}",
                col1,
                crate::confluence::wiki_escape(reason)
            )
        }

        col1
    }

    pub async fn generate(&self, description: Option<&str>) -> Result<String> {
        let data = crate::report_data::ReportData::of_slice(&[], &self.issues, 0).await?;

        let mut output = String::new();

        writeln!(&mut output, "\nh1. {}\n", self.member.name)?;
        if let Some(description) = description {
            writeln!(&mut output, "{}", description)?
        }
        writeln!(
            &mut output,
            "|| Описание таска || Эпик || Jira-таск || Сроки ||"
        )?;

        let issues: Vec<_> = self
            .issues
            .iter()
            .filter(|issue| issue.entity_type == crate::report::ReportIssueType::ReportMember)
            .collect();

        for issue in issues {
            let col1 = self.get_task(issue);
            let col2 = match &issue.custom_fields.epic_link {
                None => "",
                Some(epic_key) => match data.epics.get(&issue.jira, epic_key) {
                    None => "",
                    Some(v) => v.custom_fields.epic_name.as_deref().unwrap_or_default(),
                },
            };
            let col3 = issue.confluence_wiki_url(false);
            let col4 = issue.confluence_wiki_schedule();

            writeln!(&mut output, "| {} | {} | {} | {} |", col1, col2, col3, col4)?
        }

        Ok(output)
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Worklog {
    confluence: crate::confluence::ConfluenceServer,
    space: String,
    title: String,
    #[serde(default)]
    description: Option<String>,
    members: Vec<Member>,
}

impl Worklog {
    pub async fn make(&self) -> Result<()> {
        let mut join_set = tokio::task::JoinSet::new();
        for member in &self.members {
            let member_clone = member.clone();
            let _abort_handle = join_set.spawn(async move {
                let handler = member_clone.query_set.get_issues().await;
                (handler, member_clone)
            });
        }

        let mut members_results = Vec::new();
        while let Some(pair) = join_set.join_next().await {
            let (result, member) = pair?;
            let issues = result?;
            members_results.push(MemberResult { member, issues })
        }

        members_results.sort_by(|a, b| a.member.name.cmp(&b.member.name));

        let mut wiki_content = String::new();

        if let Some(description) = &self.description {
            writeln!(&mut wiki_content, "{}", description)?
        }

        for member_result in &members_results {
            writeln!(
                &mut wiki_content,
                "{}",
                member_result
                    .generate(
                        member_result
                            .member
                            .description
                            .as_ref()
                            .map(|v| v.as_str())
                    )
                    .await?
            )?
        }

        let get_result = self
            .confluence
            .get_content(&self.space, &self.title)
            .await
            .unwrap();

        let current_content = match get_result.results.first() {
            None => bail!("Page not found"),
            Some(v) => v,
        };

        let id: u64 = current_content.id.parse()?;

        let _result = self
            .confluence
            .update_content(
                id,
                crate::confluence_content_update::UpdateContentBody {
                    version: crate::confluence_content_update::UpdateContentBodyVersion {
                        number: current_content.version.number + 1,
                    },
                    title: current_content.title.clone(),
                    content_type: crate::confluence_types::ContentType::Page,
                    body: crate::confluence_types::ContentBody {
                        storage: crate::confluence_types::ContentBodyStorage {
                            value: wiki_content,
                            representation: crate::confluence_types::ContentRepresentation::Wiki,
                        },
                    },
                },
            )
            .await?;

        Ok(())
    }
}
