mod authentication;
mod config;
mod confluence;
mod jira;
mod jira_types;
mod printer;
mod report;
mod report_data;
mod report_dependency_graph;
mod report_roadmap;
mod serde;

extern crate slog_scope;

use anyhow::{bail, Result};
use clap::{Args, IntoApp, Parser, Subcommand};

const APP_CONFIG: &str = "uprava.yaml";

#[derive(Args, Debug)]
struct CmdJiraGetIssue {
    #[clap(short)]
    format: crate::jira::IssuePrinter,
    issue: String,
}

impl CmdJiraGetIssue {
    pub async fn run(&self, config: crate::config::Config) -> Result<()> {
        let issue = config
            .default_jira_instance
            .issue_bean(&self.issue)
            .await
            .unwrap();
        println!("{}", self.format.data_to_string(&issue).unwrap());
        Ok(())
    }
}

#[derive(Args, Debug)]
struct CmdJiraGetArbitrary {
    query: String,
}

impl CmdJiraGetArbitrary {
    pub async fn run(&self, config: crate::config::Config) -> Result<()> {
        let result = config
            .default_jira_instance
            .http_get(&self.query, &[])
            .await?;
        println!("{}", result);
        Ok(())
    }
}

#[derive(Subcommand, Debug)]
enum CmdJiraGet {
    Issue(CmdJiraGetIssue),
    Arbitrary(CmdJiraGetArbitrary),
}

impl CmdJiraGet {
    pub async fn run(&self, config: crate::config::Config) -> Result<()> {
        match self {
            CmdJiraGet::Issue(v) => v.run(config).await,
            CmdJiraGet::Arbitrary(v) => v.run(config).await,
        }
    }
}

#[derive(Args, Debug)]
struct CmdJiraSearch {
    #[clap(short)]
    format: crate::printer::SerdePrinter,
    query: String,
}

impl CmdJiraSearch {
    pub async fn run(&self, config: crate::config::Config) -> Result<()> {
        let list = config
            .default_jira_instance
            .search(&jira::SearchGetParams::new(&self.query))
            .await
            .unwrap();
        println!("{}", self.format.data_to_string(&list).unwrap());
        Ok(())
    }
}

#[derive(Subcommand, Debug)]
enum CmdJira {
    #[clap(subcommand)]
    Get(CmdJiraGet),
    Search(CmdJiraSearch),
}

impl CmdJira {
    pub async fn run(&self, config: crate::config::Config) -> Result<()> {
        match self {
            CmdJira::Get(v) => v.run(config).await,
            CmdJira::Search(v) => v.run(config).await,
        }
    }
}

#[derive(Args, Debug)]
struct CmdReportMake {
    report: String,
}

impl CmdReportMake {
    pub async fn run(&self, config: crate::config::Config) -> Result<()> {
        let report = match config.reports.get(&self.report) {
            None => bail!("Report {:?} is not defined in config file", self.report),
            Some(v) => v,
        };
        report.make().await?;

        Ok(())
    }
}

#[derive(Subcommand, Debug)]
enum CmdReport {
    Make(CmdReportMake),
}

impl CmdReport {
    pub async fn run(&self, config: crate::config::Config) -> Result<()> {
        match self {
            CmdReport::Make(v) => v.run(config).await,
        }
    }
}

#[derive(Subcommand)]
enum CmdApplication {
    #[clap(subcommand)]
    Jira(CmdJira),
    #[clap(subcommand)]
    Report(CmdReport),
    Completions {
        #[clap(arg_enum)]
        shell: clap_complete_command::Shell,
    },
}

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Application {
    #[clap(subcommand)]
    command: CmdApplication,
    #[clap(short, default_value = APP_CONFIG)]
    /// Path to configuration file
    pub config: String,
}

impl Application {
    pub async fn run_command(&self, config: crate::config::Config) -> Result<()> {
        match &self.command {
            CmdApplication::Jira(v) => v.run(config).await,
            CmdApplication::Report(v) => v.run(config).await,
            CmdApplication::Completions { shell } => {
                shell.generate(&mut Application::command(), &mut std::io::stdout());
                Ok(())
            }
        }
    }

    pub fn run(&self) {
        let _logger_guard = slog_envlogger::init().unwrap();

        let config = crate::config::Config::read(&self.config).expect("Config");

        let rt = tokio::runtime::Runtime::new().expect("Async runtime");
        rt.block_on(self.run_command(config)).expect("Runtime")
    }
}

fn main() {
    Application::parse().run()
}
