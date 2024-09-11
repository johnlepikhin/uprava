use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Write, sync::Arc};

use crate::report::ReportIssue;

#[derive(Serialize, Deserialize, Clone)]
pub enum GroupBy {
    Reporter,
    Assignee,
    Epic,
    Label,
}

impl GroupBy {
    pub fn get_column_title(&self) -> &str {
        match self {
            GroupBy::Reporter => "Автор",
            GroupBy::Assignee => "Исполнитель",
            GroupBy::Epic => "Эпик",
            GroupBy::Label => "Метка",
        }
    }

    pub fn get_titles(
        &self,
        issue: &crate::report::ReportIssue,
        data: &crate::report_data::ReportData,
    ) -> Vec<String> {
        match self {
            GroupBy::Reporter => vec![issue
                .issue
                .fields
                .creator
                .display_name
                .clone()
                .unwrap_or("не определен".to_owned())],
            GroupBy::Assignee => vec![issue
                .issue
                .fields
                .assignee
                .as_ref()
                .and_then(|v| v.display_name.clone())
                .unwrap_or("не назначен".to_owned())],
            GroupBy::Epic => vec![issue
                .custom_fields
                .epic_link
                .as_ref()
                .and_then(|epic_key| data.epics.get(&issue.jira, epic_key))
                .map(|v| v.confluence_wiki_epic_url())
                .unwrap_or_default()],
            GroupBy::Label => issue.issue.fields.labels.clone().unwrap_or_default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Member {
    name: String,
    story_points_field: String,
    group_by: GroupBy,
    query_set: crate::report::QuerySet,
    #[serde(default)]
    description: Option<String>,
}

pub struct MemberResult {
    member: Member,
    issues: Vec<ReportIssue>,
}

impl MemberResult {
    pub async fn generate(&self) -> Result<String> {
        let data = crate::report_data::ReportData::of_slice(&[], &self.issues, 0).await?;

        let mut output = String::new();

        writeln!(&mut output, "\nh1. {}\n", self.member.name)?;
        if let Some(description) = &self.member.description {
            writeln!(&mut output, "{}", description)?
        }
        writeln!(
            &mut output,
            "|| {} || Сторипоинты ||",
            self.member.group_by.get_column_title()
        )?;

        let mut sums = HashMap::new();
        for issue in &self.issues {
            let titles = self.member.group_by.get_titles(issue, &data);
            let story_points = issue
                .custom_field_f64(&self.member.story_points_field)
                .unwrap_or_default();
            for title in titles {
                let ent = sums.entry(title).or_insert(0);
                *ent += story_points as i64;
            }
        }

        let mut keys = sums.keys().collect::<Vec<_>>();
        keys.sort_by(|a, b| sums.get(b.as_str()).cmp(&sums.get(a.as_str())));

        for key in keys {
            writeln!(&mut output, "| {} | {} |", key, sums[key])?
        }

        Ok(output)
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct StoryPoints {
    confluence: crate::confluence::ConfluenceServer,
    space: String,
    title: String,
    #[serde(default)]
    description: Option<String>,
    members: Vec<Member>,
}

impl StoryPoints {
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
            members_results.push(MemberResult { member, issues })
        }

        // Results are asyncronous so should be sorted for stable ordering
        members_results.sort_by(|a, b| a.member.name.cmp(&b.member.name));

        let mut wiki_content = String::new();

        if let Some(description) = &self.description {
            writeln!(&mut wiki_content, "{}", description)?
        }

        for member_result in &members_results {
            writeln!(&mut wiki_content, "{}", member_result.generate().await?)?
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
