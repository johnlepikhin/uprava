use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::{fmt::Write, sync::Arc};

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
    show_author: bool,
    show_assignee: bool,
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
        if self.show_author {
            slog_scope::debug!("Author: {:?}", issue.issue.fields.creator);
            col1 = format!(
                "{}\\\\ \\\\Автор: {}",
                col1,
                crate::confluence::wiki_escape(
                    issue
                        .issue
                        .fields
                        .creator
                        .display_name
                        .as_deref()
                        .unwrap_or("не определен")
                )
            )
        }

        if self.show_assignee {
            slog_scope::debug!("Assignee: {:?}", issue.issue.fields.assignee);
            if let Some(assignee) = &issue.issue.fields.assignee {
                col1 = format!(
                    "{}\\\\ \\\\Исполнитель: {}",
                    col1,
                    crate::confluence::wiki_escape(
                        assignee.display_name.as_deref().unwrap_or("не назначен")
                    )
                )
            }
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
                None => String::new(),
                Some(epic_key) => match data.epics.get(&issue.jira, epic_key) {
                    None => String::new(),
                    Some(v) => v.confluence_wiki_epic_url(),
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
    #[serde(default)]
    show_author: bool,
    #[serde(default)]
    show_assignee: bool,
    members: Vec<Member>,
}

impl Worklog {
    pub async fn make(&self, config: Arc<crate::config::Config>) -> Result<()> {
        let mut join_set = tokio::task::JoinSet::new();
        for member in &self.members {
            let member_clone = member.clone();
            let config = config.clone();
            let _abort_handle = join_set.spawn(async move {
                let handler = member_clone.query_set.get_issues(config).await;
                (handler, member_clone)
            });
        }

        let mut members_results = Vec::new();
        while let Some(pair) = join_set.join_next().await {
            let (result, member) = pair?;
            let issues = result?;
            members_results.push(MemberResult {
                member,
                issues,
                show_author: self.show_author,
                show_assignee: self.show_assignee,
            })
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
                    .generate(member_result.member.description.as_deref())
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
