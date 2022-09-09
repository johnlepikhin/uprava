use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Roadmap;

impl Roadmap {
    fn get_task(&self, issue: &crate::report::ReportIssue) -> String {
        let mut col1 = format!(
            " *{}*",
            crate::confluence::wiki_escape(&issue.issue.fields.summary)
        );
        if let Some(reason) = &issue.custom_fields.reason {
            col1 = format!(
                "{}\\\\ \\\\{}",
                col1,
                crate::confluence::wiki_escape(&reason)
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

    fn get_issue_link(&self, issue: &crate::report::ReportIssue) -> String {
        let mut issue_url = issue.jira.base_url.clone();
        issue_url.set_path(&format!("browse/{}", issue.issue.key));
        format!(
            "[{}|{}] \\\\ {}",
            issue.issue.key,
            issue_url,
            crate::confluence::wiki_escape(
                &issue
                    .issue
                    .fields
                    .status
                    .as_ref()
                    .map(|v| v.name.clone())
                    .flatten()
                    .unwrap_or_default()
            )
        )
    }

    pub async fn make(&self, data: &crate::report_data::ReportData) -> Result<()> {
        println!("\nh1. Задачи\n");
        println!("|| Описание таска || Эпик || Jira-таск || Сроки ||");

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

        for issue in issues {
            let col1 = self.get_task(issue);
            let col2 = match &issue.custom_fields.epic_link {
                None => "",
                Some(epic_key) => match data.epics.get(&issue.jira, &epic_key) {
                    None => "",
                    Some(v) => v.custom_fields.epic_name.as_deref().unwrap_or_default(),
                },
            };

            let col3 = self.get_issue_link(issue);

            let duration = issue.custom_fields.plan();

            let duration = {
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
            };

            let col4 = duration;

            println!("| {} | {} | {} | {} |", col1, col2, col3, col4)
        }

        println!("\nh1. Эпики\n");
        println!("|| Эпик || Описание эпика ||");
        for epic in data.epics.all().values() {
            if let Some(epic_name) = &epic.custom_fields.epic_name {
                let mut issue_url = epic.jira.base_url.clone();
                issue_url.set_path(&format!("browse/{}", epic.issue.key));

                println!(
                    "| [{}|{}] | {} |",
                    crate::confluence::wiki_escape(&epic_name),
                    issue_url,
                    crate::confluence::wiki_escape(
                        &epic.custom_fields.reason.as_deref().unwrap_or("")
                    ),
                )
            }
        }

        Ok(())
    }
}
