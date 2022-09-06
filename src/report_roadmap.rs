use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Roadmap;

impl Roadmap {
    pub async fn make(&self, issues: &[crate::report::ReportIssue]) -> Result<()> {
        println!("\nh1. Задачи\n\n");
        println!("|| Описание таска || Эпик || Jira-таск || Сроки ||\n");
        for issue in issues {
            let mut col1 = format!(" *{}*", issue.issue.fields.summary);
            if let Some(reason) = issue.custom_fields.get("reason") {
                if let serde_json::Value::String(v) = reason {
                    col1 = format!("{}\\\\ \\\\{}", col1, v)
                }
            }
            println!("|{}|", col1)
        }

        Ok(())
    }
}
