use std::collections::HashSet;
use std::fmt::Write;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::report_data::IssueID;

#[derive(Serialize, Deserialize, Clone)]
pub struct ConfluenceRoadmap {
    confluence: crate::confluence::ConfluenceServer,
    space: String,
    title: String,
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

    fn get_issue_link(&self, issue: &crate::report::ReportIssue, newlines: bool) -> String {
        let mut issue_url = issue.jira.base_url.clone();
        issue_url.set_path(&format!("browse/{}", issue.issue.key));
        format!(
            "[{}|{}] {} {}",
            issue.issue.key,
            issue_url,
            if newlines { "\\\\" } else { "" },
            crate::confluence::wiki_escape(
                &issue
                    .issue
                    .fields
                    .status
                    .as_ref()
                    .and_then(|v| v.name.clone())
                    .unwrap_or_default()
            )
        )
    }

    fn get_issue_plan(&self, issue: &crate::report::ReportIssue) -> String {
        let duration = issue.custom_fields.plan();
        let planned_end = issue
            .custom_fields
            .planned_end
            .unwrap_or_else(|| chrono::Utc::now().date() + chrono::Duration::days(100000));
        let planned_start = issue
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
                    None => "",
                    Some(epic_key) => match data.epics.get(&issue.jira, epic_key) {
                        None => "",
                        Some(v) => v.custom_fields.epic_name.as_deref().unwrap_or_default(),
                    },
                };
                let col3 = self.get_issue_link(issue, false);
                let col4 = self.get_issue_plan(issue);

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
        writeln!(&mut result, "|| Эпик || Описание эпика ||")?;

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

        for epic in data.epics.all().iter().filter_map(|(k, v)| {
            if local_epics.contains(k) {
                Some(v)
            } else {
                None
            }
        }) {
            if let Some(epic_name) = &epic.custom_fields.epic_name {
                let mut issue_url = epic.jira.base_url.clone();
                issue_url.set_path(&format!("browse/{}", epic.issue.key));

                writeln!(
                    &mut result,
                    "| [{}|{}] | {} |",
                    crate::confluence::wiki_escape(epic_name),
                    issue_url,
                    crate::confluence::wiki_escape(
                        epic.custom_fields.reason.as_deref().unwrap_or("")
                    ),
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
                None => "",
                Some(epic_key) => match data.epics.get(&issue.jira, epic_key) {
                    None => "",
                    Some(v) => v.custom_fields.epic_name.as_deref().unwrap_or_default(),
                },
            };

            let col3 = self.get_issue_link(issue, true);
            let col4 = self.get_issue_plan(issue);

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

        let mut issues: Vec<_> = data
            .issues
            .all()
            .values()
            .filter(|issue| issue.entity_type == crate::report::ReportIssueType::ReportMember)
            .collect();
        issues.sort_by(|a, b| {
            a.custom_fields
                .planned_end
                .is_none()
                .cmp(&b.custom_fields.planned_end.is_none())
                .then_with(|| {
                    a.custom_fields
                        .planned_end
                        .cmp(&b.custom_fields.planned_end)
                        .then_with(|| {
                            a.custom_fields
                                .planned_start
                                .is_none()
                                .cmp(&b.custom_fields.planned_start.is_none())
                                .then_with(|| {
                                    a.custom_fields
                                        .planned_start
                                        .cmp(&b.custom_fields.planned_start)
                                        .then_with(|| {
                                            a.issue.fields.created.cmp(&b.issue.fields.created)
                                        })
                                })
                        })
                })
        });

        writeln!(
            &mut output,
            "{}",
            self.make_general_list(data, issues.as_slice())?
        )?;
        writeln!(&mut output, "{}", self.make_epics(data, issues.as_slice())?)?;
        writeln!(
            &mut output,
            "{}",
            self.make_team_roadmaps(data, issues.as_slice())?
        )?;

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

    pub async fn make(&self, data: &crate::report_data::ReportData) -> Result<()> {
        let wiki_content = self.generate(data)?;

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

        self.upload_dependency_graph(id, data).await?;

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
