use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Hash, PartialEq, Eq, Debug)]
pub struct ConfluenceServer {
    pub base_url: url::Url,
    pub access: crate::authentication::Access,
}

impl ConfluenceServer {
    pub async fn http_get(&self, path: &str, params: &[(&str, &str)]) -> Result<String> {
        let http_client = reqwest::ClientBuilder::new()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;
        let mut url = self.base_url.clone();
        url.set_path(path);
        url.query_pairs_mut().extend_pairs(params);

        let request = http_client
            .get(url.clone())
            .header(reqwest::header::ACCEPT, "application/json")
            .header(reqwest::header::CONTENT_TYPE, "application/json");
        let request = match &self.access {
            crate::authentication::Access::Token(secret) => request.header(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", secret.get()?.trim()),
            ),
            crate::authentication::Access::JSessionID(secret) => request.header(
                reqwest::header::COOKIE,
                format!("JSESSIONID={}", secret.get()?.trim()),
            ),
        };

        let response = request.send().await?.error_for_status()?.text().await?;

        Ok(response)
    }

    pub async fn get_content(&self, space: &str, title: &str) -> Result<String> {
        slog_scope::info!(
            "Getting content from {:?}: space {:?} title {:?}",
            self.base_url,
            space,
            title
        );

        let query = vec![("spaceKey", space), ("title", title)];

        let response = self
            .http_get(&format!("/rest/api/content"), query.as_slice())
            .await?;

        slog_scope::trace!("Got from {:?}: {:?}", self.base_url, response);

        Ok(response)
    }

    // https://stackoverflow.com/questions/23523705/how-to-create-new-page-in-confluence-using-their-rest-api
    // https://docs.atlassian.com/ConfluenceServer/rest/7.12.3/#api/content-getContent
}

pub fn wiki_escape(s: &str) -> String {
    s.trim()
        .replace('\r', "")
        .replace('\n', "\\\\")
        .replace('{', "")
        .replace('}', "")
}
