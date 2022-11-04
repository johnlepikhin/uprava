use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Hash, PartialEq, Eq, Debug)]
pub struct ConfluenceServer {
    pub base_url: url::Url,
    #[serde(with = "serde_yaml::with::singleton_map")]
    pub access: crate::authentication::Access,
}

impl ConfluenceServer {
    pub fn http_client(&self) -> Result<reqwest::Client> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::ACCEPT,
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        match &self.access {
            crate::authentication::Access::Token(secret) => {
                let mut secret = reqwest::header::HeaderValue::from_str(&format!(
                    "Bearer {}",
                    secret.get()?.trim()
                ))?;
                secret.set_sensitive(true);
                let _ = headers.insert(reqwest::header::AUTHORIZATION, secret);
            }
            crate::authentication::Access::JSessionID(secret) => {
                let mut secret = reqwest::header::HeaderValue::from_str(&format!(
                    "JSESSIONID={}",
                    secret.get()?.trim()
                ))?;
                secret.set_sensitive(true);
                let _ = headers.insert(reqwest::header::COOKIE, secret);
            }
        }

        let http_client = reqwest::ClientBuilder::new()
            .timeout(std::time::Duration::from_secs(30))
            .default_headers(headers)
            .build()?;

        Ok(http_client)
    }

    pub async fn http_get(&self, path: &str, params: &[(&str, &str)]) -> Result<String> {
        let http_client = self.http_client()?;

        let mut url = self.base_url.clone();
        url.set_path(path);
        url.query_pairs_mut().extend_pairs(params);

        let response = http_client
            .get(url.clone())
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        Ok(response)
    }

    pub async fn http_put(
        &self,
        path: &str,
        params: &[(&str, &str)],
        body: String,
    ) -> Result<String> {
        let http_client = self.http_client()?;

        let mut url = self.base_url.clone();
        url.set_path(path);
        url.query_pairs_mut().extend_pairs(params);

        let response = http_client
            .put(url.clone())
            .body(body)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        Ok(response)
    }

    pub async fn get_content(
        &self,
        space: &str,
        title: &str,
    ) -> Result<crate::confluence_types::PagedResult<crate::confluence_content_get::GetResult>>
    {
        slog_scope::info!(
            "Getting content from {:?}: space {:?} title {:?}",
            self.base_url,
            space,
            title
        );

        let query = vec![
            ("spaceKey", space),
            ("title", title),
            ("expand", "body.storage,version"),
        ];

        let response = self.http_get("/rest/api/content", query.as_slice()).await?;

        slog_scope::trace!("Got from {:?}: {:?}", self.base_url, response);

        let json = serde_json::de::from_str::<
            crate::confluence_types::PagedResult<crate::confluence_content_get::GetResult>,
        >(&response)?;

        Ok(json)
    }

    pub async fn update_content(
        &self,
        content_id: u64,
        content: crate::confluence_content_update::UpdateContentBody,
    ) -> Result<String> {
        slog_scope::info!(
            "Setting content in {:?} for ID {:?}, new title {:?}",
            self.base_url,
            content_id,
            content.title,
        );

        let response = self
            .http_put(
                &format!("/rest/api/content/{}", content_id),
                &[],
                serde_json::to_string(&content)?,
            )
            .await?;

        Ok(response)
    }

    async fn file_part<T: AsRef<std::path::Path>>(
        path: T,
        filename: &str,
    ) -> Result<reqwest::multipart::Part> {
        use reqwest::multipart::Part;

        let path = path.as_ref();
        let ext = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");
        let mime = mime_guess::from_ext(ext).first_or_octet_stream();
        let file = tokio::fs::File::open(path).await?;
        let mime_type = format!("{}/{}", mime.type_(), mime.subtype());
        slog_scope::debug!("Detected MIME type: {}", mime_type);
        let field = Part::stream(file).mime_str(&mime_type)?;

        Ok(field.file_name(filename.to_owned()))
    }

    pub async fn upload_attachment(
        &self,
        content_id: u64,
        file_path: &std::path::Path,
        filename: &str,
    ) -> Result<String> {
        use reqwest::multipart;

        let http_client = self.http_client()?;

        let mut url = self.base_url.clone();
        url.set_path(&format!(
            "/rest/api/content/{}/child/attachment",
            content_id
        ));
        url.query_pairs_mut()
            .extend_pairs(&[("allowDuplicated", "true")]);

        let form = multipart::Form::new().part("file", Self::file_part(file_path, filename).await?);

        let response = http_client
            .post(url)
            .header("X-Atlassian-Token", "nocheck")
            .multipart(form)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        Ok(response)
    }
}

pub fn wiki_escape(s: &str) -> String {
    s.trim()
        .replace('\r', "")
        .replace('\n', "\\\\")
        .replace('{', "")
        .replace('}', "")
}
