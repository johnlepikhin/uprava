use std::collections::HashSet;

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct DependencyGraph;

impl DependencyGraph {
    fn issue_id(issue: &crate::report::ReportIssue) -> String {
        format!("{}/{}", issue.jira.base_url, issue.issue.key)
            .replace(':', "_")
            .replace('/', "_")
            .replace('-', "_")
            .replace('.', "_")
    }

    fn double_string_escape(s: &str) -> String {
        s.replace('\\', "\\\\").replace('"', "\\\"")
    }

    fn html_string_escape(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    }

    pub fn generate_dot(&self, data: &crate::report_data::ReportData) -> Result<String> {
        use std::fmt::Write;

        let mut output = String::new();
        writeln!(&mut output, "digraph dependency_graph {{")?;
        writeln!(&mut output, "graph [layout=dot, rankdir=LR, ranksep=1.2]")?;
        writeln!(&mut output, "node [style=filled, shape=box]")?;
        writeln!(&mut output, "edge [penwidth=2]")?;

        let issues_epics: HashSet<_> = data
            .issues
            .all()
            .values()
            .map(|issue| (&issue.jira, &issue.custom_fields.epic_link))
            .collect();

        for (cluster_id, (jira, epic_link)) in issues_epics.iter().enumerate() {
            let epic_issues =
                data.issues.all().values().filter(|issue| {
                    (&issue.jira, &issue.custom_fields.epic_link) == (jira, epic_link)
                });

            let epic = match &epic_link {
                None => None,
                Some(epic_link) => {
                    slog_scope::debug!("Searching for epic {}", &epic_link);
                    let epic_id = crate::report_data::IssueID::new(jira, epic_link);
                    data.epics.all().get(&epic_id)
                }
            };

            if let Some(epic) = epic {
                let epic_summary = epic.issue.fields.summary.as_str();
                let epic_href = format!("; href=\"{}\"", epic.url());
                writeln!(
                    &mut output,
                    " subgraph cluster_{} {{ style=filled; color=\"#C0D5FF\"; label=\"ЭПИК: {}\"{}",
                    cluster_id, epic_summary, epic_href
                )?;
            }

            for issue in epic_issues {
                let duration = if issue.custom_fields.planned_start.is_some()
                    || issue.custom_fields.planned_end.is_some()
                {
                    format!("<br/>План: {}", issue.custom_fields.plan())
                } else {
                    "".to_owned()
                };
                let duration = {
                    let planned_end = issue.custom_fields.planned_end.unwrap_or_else(|| {
                        chrono::Utc::now().date() + chrono::Duration::days(100000)
                    });
                    let planned_start = issue.custom_fields.planned_start.unwrap_or_else(|| {
                        chrono::Utc::now().date() + chrono::Duration::days(100000)
                    });
                    if planned_end - chrono::Duration::days(3) < chrono::Utc::now().date() {
                        format!("<font color=\"red\">{}</font>", duration)
                    } else if planned_start - chrono::Duration::days(3) < chrono::Utc::now().date()
                    {
                        format!("<font color=\"green\">{}</font>", duration)
                    } else {
                        duration
                    }
                };

                let assignee = match &issue.issue.fields.assignee {
                    None => "".to_owned(),
                    Some(v) => match &v.display_name {
                        None => "".to_owned(),
                        Some(v) => format!("<br/>Исполнитель {}", Self::html_string_escape(v)),
                    },
                };

                let status = issue
                    .issue
                    .fields
                    .status
                    .as_ref()
                    .and_then(|v| {
                        v.name
                            .as_ref()
                            .map(|v| format!("<br/>{}", Self::html_string_escape(v)))
                    })
                    .unwrap_or_default();

                let status_color = match issue
                    .issue
                    .fields
                    .status
                    .as_ref()
                    .and_then(|v| v.status_category.as_ref().map(|v| v.key.as_deref()))
                    .flatten()
                {
                    Some("new") => "#42526e",
                    Some("done") => "green",
                    Some("indeterminate") => "blue",
                    _ => "red",
                };

                match issue.entity_type {
                    crate::report::ReportIssueType::ReportMember => writeln!(
                        output,
                        "    {} [fillcolor=\"#8CB3FF\";label=<{}{}{}<i><font color=\"{}\">{}</font></i>>;href=\"{}\"]",
                        Self::issue_id(issue),
                        &Self::html_string_escape(&issue.issue.fields.summary),
                        &assignee,
                        &duration,
                        status_color,
                        &status,
                        Self::double_string_escape(issue.url().as_ref()),
                    )?,
                    crate::report::ReportIssueType::ExternalDependency => writeln!(
                        output,
                        "    {} [fillcolor=\"#80FFD2\";href=\"{}\";label=<Внешняя задача<br/>{}{}{}<i><font color=\"{}\">{}</font></i>>]",
                        Self::issue_id(issue),
                        Self::double_string_escape(issue.url().as_ref()),
                        &Self::html_string_escape(&issue.issue.fields.summary),
                        &assignee,
                        &duration,
                        status_color,
                        &status,
                    )?,
                    crate::report::ReportIssueType::Epic => (),
                }
            }

            if epic.is_some() {
                writeln!(&mut output, "  }}")?;
            }
        }

        for relation in &data.relations {
            let style_attrs = match relation.kind {
                crate::report::RelationKind::Dependance => "color=\"#2E56A6\", style=solid",
                crate::report::RelationKind::Block => "color=\"#A65229\", style=bold",
                crate::report::RelationKind::Mention => "color=\"#7F94BF\", style=dashed",
            };
            writeln!(
                &mut output,
                "{} -> {} [{}]",
                relation.from.as_string(),
                relation.to.as_string(),
                style_attrs
            )?;
        }
        writeln!(&mut output, "}}")?;

        Ok(output)
    }

    pub fn make(&self, data: &crate::report_data::ReportData) -> Result<tempfile::NamedTempFile> {
        use std::io::Write;

        let dotfile_content = self.generate_dot(data)?;

        let mut dotfile = tempfile::NamedTempFile::new()?;
        dotfile.write_all(dotfile_content.as_bytes())?;

        let svg_file = tempfile::NamedTempFile::new()?;

        std::process::Command::new("dot")
            .args(&[
                "-Tsvg",
                "-o",
                svg_file.path().to_str().unwrap(),
                dotfile.path().to_str().unwrap(),
            ])
            .output()
            .map_err(|err| {
                anyhow::format_err!(
                    "Failed to execute 'dot' command of Graphviz project: {}",
                    err
                )
            })?;

        Ok(svg_file)
    }
}

// Color scheme: https://colorscheme.ru/#36422g0--w0w0
