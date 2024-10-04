mod authentication;
mod config;
mod confluence;
mod confluence_content_get;
mod confluence_content_update;
mod confluence_types;
mod jira;
mod jira_types;
mod printer;
mod report;
mod report_confluence_roadmap;
mod report_data;
mod report_dependency_graph;
mod report_storypoints;
mod report_worklog;
mod serde;

extern crate slog_scope;

use std::{io::Read, sync::Arc};

use anyhow::{bail, Result};
use clap::{Args, CommandFactory, Parser, Subcommand};

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
        let url = url::Url::parse(&format!("http://a/{}", self.query))?;

        let params: Vec<_> = url
            .query_pairs()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();

        let params: Vec<_> = params
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();

        let result = config
            .default_jira_instance
            .http_get(url.path().to_string().as_str(), &params)
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
struct CmdConfluenceGetContent {
    #[clap(short)]
    format: crate::confluence_content_get::ContentPrinter,
    space: String,
    title: String,
}

impl CmdConfluenceGetContent {
    pub async fn run(&self, config: crate::config::Config) -> Result<()> {
        let content = config
            .default_confluence_instance
            .get_content(&self.space, &self.title)
            .await
            .unwrap();

        let result = match content.results.first() {
            None => bail!("No results found"),
            Some(v) => v,
        };

        println!("{}", self.format.data_to_string(result).unwrap());

        Ok(())
    }
}

#[derive(Args, Debug)]
struct CmdConfluenceGetArbitrary {
    query: String,
}

impl CmdConfluenceGetArbitrary {
    pub async fn run(&self, config: crate::config::Config) -> Result<()> {
        let url = url::Url::parse(&format!("http://a/{}", self.query))?;

        let params: Vec<_> = url
            .query_pairs()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();

        let params: Vec<_> = params
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();

        let result = config
            .default_confluence_instance
            .http_get(url.path().to_string().as_str(), &params)
            .await?;
        println!("{}", result);
        Ok(())
    }
}

#[derive(Subcommand, Debug)]
enum CmdConfluenceGet {
    Content(CmdConfluenceGetContent),
    Arbitrary(CmdConfluenceGetArbitrary),
}

impl CmdConfluenceGet {
    pub async fn run(&self, config: crate::config::Config) -> Result<()> {
        match self {
            CmdConfluenceGet::Content(v) => v.run(config).await,
            CmdConfluenceGet::Arbitrary(v) => v.run(config).await,
        }
    }
}

#[derive(Debug, Args)]
struct CmdConfluenceUpdateWiki {
    space: String,
    title: String,
}

impl CmdConfluenceUpdateWiki {
    pub async fn run(&self, config: crate::config::Config) -> Result<()> {
        let get_result = config
            .default_confluence_instance
            .get_content(&self.space, &self.title)
            .await
            .unwrap();

        let current_content = match get_result.results.first() {
            None => bail!("No results found"),
            Some(v) => v,
        };

        let id: u64 = current_content.id.parse()?;

        let mut stdin = std::io::stdin().lock();
        let mut new_body = String::new();
        stdin.read_to_string(&mut new_body)?;

        let _result = config
            .default_confluence_instance
            .update_content(
                id,
                confluence_content_update::UpdateContentBody {
                    version: confluence_content_update::UpdateContentBodyVersion {
                        number: current_content.version.number + 1,
                    },
                    title: current_content.title.clone(),
                    content_type: confluence_types::ContentType::Page,
                    body: confluence_types::ContentBody {
                        storage: confluence_types::ContentBodyStorage {
                            value: new_body,
                            representation: confluence_types::ContentRepresentation::Wiki,
                        },
                    },
                },
            )
            .await?;

        Ok(())
    }
}

impl CmdConfluenceUploadFile {
    pub async fn run(&self, config: crate::config::Config) -> Result<()> {
        let get_result = config
            .default_confluence_instance
            .get_content(&self.space, &self.title)
            .await
            .unwrap();

        let current_content = match get_result.results.first() {
            None => bail!("No results found"),
            Some(v) => v,
        };

        let id: u64 = current_content.id.parse()?;

        let _result = config
            .default_confluence_instance
            .upload_attachment(id, &self.path, &self.filename)
            .await
            .unwrap();

        Ok(())
    }
}

#[derive(Debug, Args)]
struct CmdConfluenceUploadFile {
    space: String,
    title: String,
    path: std::path::PathBuf,
    filename: String,
}

#[derive(Subcommand, Debug)]
enum CmdConfluence {
    #[clap(subcommand)]
    Get(CmdConfluenceGet),
    UpdateWiki(CmdConfluenceUpdateWiki),
    UploadFile(CmdConfluenceUploadFile),
}

impl CmdConfluence {
    pub async fn run(&self, config: crate::config::Config) -> Result<()> {
        match self {
            CmdConfluence::Get(v) => v.run(config).await,
            CmdConfluence::UpdateWiki(v) => v.run(config).await,
            CmdConfluence::UploadFile(v) => v.run(config).await,
        }
    }
}

#[derive(Args, Debug)]
struct CmdReportMake {
    report: String,
}

impl CmdReportMake {
    pub async fn run(&self, config: Arc<crate::config::Config>) -> Result<()> {
        let report = match config.reports.get(&self.report) {
            None => bail!("Report {:?} is not defined in config file", self.report),
            Some(v) => v.clone(),
        };
        match &report.0 {
            report::Report::ConfluenceRoadmap(v) => v.make(config).await?,
            report::Report::Worklog(v) => v.make(config).await?,
            report::Report::StoryPoints(v) => v.make(config).await?,
        }

        Ok(())
    }
}

#[derive(Subcommand, Debug)]
enum CmdReport {
    Make(CmdReportMake),
    MakeAll,
    List,
}

impl CmdReport {
    pub async fn run(&self, config: crate::config::Config) -> Result<()> {
        let config = Arc::new(config);
        match self {
            CmdReport::Make(v) => v.run(config).await,
            CmdReport::MakeAll => {
                for (name, _) in config.reports.iter() {
                    slog_scope::info!("Running report {:?}", name);
                    let report = CmdReportMake {
                        report: name.clone(),
                    };
                    report.run(config.clone()).await?;
                }
                Ok(())
            }
            CmdReport::List => {
                let mut names: Vec<_> = config.reports.keys().collect();
                names.sort();
                for name in names {
                    println!("{}", name);
                }
                Ok(())
            }
        }
    }
}

#[derive(Subcommand)]
enum CmdApplication {
    #[clap(subcommand)]
    Jira(CmdJira),
    #[clap(subcommand)]
    Confluence(CmdConfluence),
    #[clap(subcommand)]
    Report(CmdReport),
    Completions {
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
            CmdApplication::Confluence(v) => v.run(config).await,
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
