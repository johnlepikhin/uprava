use std::fmt::Write;
use std::{collections::HashSet, sync::Arc};

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::report_data::IssueID;

fn default_show_epics() -> bool {
    true
}

fn default_show_team_roadmaps() -> bool {
    true
}

fn default_dependencies_deepness() -> usize {
    1
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ConfluenceRoadmap {
    query_set: crate::report::QuerySet,
    confluence: crate::confluence::ConfluenceServer,
    space: String,
    title: String,
    #[serde(default = "default_show_epics")]
    show_epics: bool,
    #[serde(default = "default_show_team_roadmaps")]
    show_team_roadmaps: bool,
    #[serde(default = "default_dependencies_deepness")]
    dependencies_deepness: usize,
    #[serde(default)]
    foreign_relations: Vec<crate::report::ForeignRelation>,
}

impl ConfluenceRoadmap {
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
        if let Some(assignee) = &issue.issue.fields.assignee {
            col1 = format!("{}\\\\ \\\\ Исполнитель", col1);
            if let Some(display_name) = &assignee.display_name {
                col1 = format!("{} {}", col1, crate::confluence::wiki_escape(display_name));
            }
            if !assignee.active.unwrap_or(true) {
                col1 = format!("{} *не активен!*", col1)
            }
        }

        col1
    }

    fn make_team_roadmaps(
        &self,
        data: &crate::report_data::ReportData,
        issues: &[&crate::report::ReportIssue],
    ) -> Result<String> {
        let mut result = String::new();
        writeln!(&mut result, "\nh1. Команда\n")?;
        let local_assignees: HashSet<_> = issues
            .iter()
            .map(|issue| {
                issue
                    .issue
                    .fields
                    .assignee
                    .as_ref()
                    .and_then(|user| user.display_name.as_deref())
            })
            .collect();
        let mut local_assignees: Vec<_> = local_assignees.into_iter().collect();
        local_assignees.sort();

        for assignee in local_assignees {
            let assignee_issues: Vec<_> = issues
                .iter()
                .filter(|issue| {
                    let issue_assignee = issue
                        .issue
                        .fields
                        .assignee
                        .as_ref()
                        .and_then(|user| user.display_name.as_deref());
                    issue_assignee == assignee
                })
                .collect();

            writeln!(
                &mut result,
                "\nh2. {} ({} задач)\n",
                assignee.unwrap_or("Без исполнителя"),
                assignee_issues.len()
            )?;
            writeln!(
                &mut result,
                "|| Описание таска || Эпик || Jira-таск || Сроки ||"
            )?;
            for issue in assignee_issues {
                let col1 = crate::confluence::wiki_escape(&issue.issue.fields.summary);
                let col2 = match &issue.custom_fields.epic_link {
                    None => {
                        slog_scope::debug!("epic_link was empty");
                        String::new()
                    }
                    Some(epic_key) => {
                        slog_scope::debug!("epic_key is {}", epic_key);
                        match data.epics.get(&issue.jira, epic_key) {
                            None => {
                                slog_scope::debug!("linked epic was not found");
                                String::new()
                            }
                            Some(v) => v.confluence_wiki_epic_url(),
                        }
                    }
                };
                let col3 = issue.confluence_wiki_url(false);
                let col4 = issue.confluence_wiki_schedule();

                writeln!(&mut result, "| {} | {} | {} | {} |", col1, col2, col3, col4)?
            }
        }
        Ok(result)
    }

    fn make_epics(
        &self,
        data: &crate::report_data::ReportData,
        issues: &[&crate::report::ReportIssue],
    ) -> Result<String> {
        let mut result = String::new();
        writeln!(&mut result, "\nh1. Эпики\n")?;
        writeln!(&mut result, "|| Эпик || Описание эпика || Обоснование ||")?;

        let local_epics: HashSet<_> = issues
            .iter()
            .filter_map(|issue| {
                issue
                    .custom_fields
                    .epic_link
                    .as_ref()
                    .map(|key| IssueID::new(&issue.jira, key))
            })
            .collect();

        slog_scope::debug!("Roadmap report got {} epics", data.epics.all().len());
        for epic in data.epics.all().iter().filter_map(|(k, v)| {
            if local_epics.contains(k) {
                Some(v)
            } else {
                slog_scope::debug!("Skipping epic {}, doesn't present in local_epics", k.issue);
                None
            }
        }) {
            if let Some(epic_name) = &epic.custom_fields.epic_name {
                let mut issue_url = epic.jira.base_url.clone();
                issue_url.set_path(&format!("browse/{}", epic.issue.key));

                writeln!(
                    &mut result,
                    "| [{}|{}] | {} | {} |",
                    crate::confluence::wiki_escape(epic_name),
                    issue_url,
                    crate::confluence::wiki_escape(epic.issue.fields.summary.as_str()),
                    crate::confluence::wiki_escape(
                        epic.custom_fields.reason.as_deref().unwrap_or_default()
                    )
                )?
            }
        }
        Ok(result)
    }

    fn make_general_list(
        &self,
        data: &crate::report_data::ReportData,
        issues: &[&crate::report::ReportIssue],
    ) -> Result<String> {
        let mut result = String::new();

        for issue in issues {
            let col1 = self.get_task(issue);
            let col2 = match &issue.custom_fields.epic_link {
                None => String::new(),
                Some(epic_key) => match data.epics.get(&issue.jira, epic_key) {
                    None => String::new(),
                    Some(v) => v.confluence_wiki_epic_url(),
                },
            };

            let col3 = issue.confluence_wiki_url(true);
            let col4 = issue.confluence_wiki_schedule();

            writeln!(&mut result, "| {} | {} | {} | {} |", col1, col2, col3, col4)?
        }

        Ok(result)
    }

    pub fn generate(&self, data: &crate::report_data::ReportData) -> Result<String> {
        let mut output = String::new();

        writeln!(&mut output, "\nh1. Задачи\n")?;
        writeln!(
            &mut output,
            "|| Описание таска || Эпик || Jira-таск || Сроки ||"
        )?;

        let issues: Vec<_> = data
            .issues
            .all()
            .values()
            .filter(|issue| issue.entity_type == crate::report::ReportIssueType::ReportMember)
            .collect();
        writeln!(
            &mut output,
            "{}",
            self.make_general_list(data, issues.as_slice())?
        )?;

        if self.show_epics {
            writeln!(&mut output, "{}", self.make_epics(data, issues.as_slice())?)?;
        }

        if self.show_team_roadmaps {
            writeln!(
                &mut output,
                "{}",
                self.make_team_roadmaps(data, issues.as_slice())?
            )?;
        }

        writeln!(&mut output, "\nh1. Граф зависимостей\n")?;
        writeln!(&mut output, "!dependency_graph.svg!")?;

        Ok(output)
    }

    pub async fn upload_dependency_graph(
        &self,
        page_id: u64,
        data: &crate::report_data::ReportData,
    ) -> Result<()> {
        let generator = crate::report_dependency_graph::DependencyGraph;
        let svg = generator.make(data)?;
        self.confluence
            .upload_attachment(page_id, svg.path(), "dependency_graph.svg")
            .await?;
        Ok(())
    }

    pub async fn make(&self, config: Arc<crate::config::Config>) -> Result<()> {
        let issues_list = self.query_set.get_issues(config).await?;
        let data = crate::report_data::ReportData::of_slice(
            &self.foreign_relations,
            &issues_list,
            self.dependencies_deepness,
        )
        .await?;

        let wiki_content = self.generate(&data)?;

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

        self.upload_dependency_graph(id, &data).await?;

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
