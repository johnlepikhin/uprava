use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, Serialize)]
pub struct ContentVersion {
    pub when: chrono::DateTime<chrono::Utc>,
    pub number: u64,
    #[serde(rename = "minorEdit")]
    pub minor_edit: bool,
    pub hidden: bool,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct GetResult {
    pub id: String,
    #[serde(rename = "type")]
    pub content_type: crate::confluence_types::ContentType,
    pub status: String,
    pub title: String,
    pub body: crate::confluence_types::ContentBody,
    pub version: ContentVersion,
}

#[derive(Debug, Clone)]
pub enum ContentPrinter {
    Email,
    Serde(crate::printer::SerdePrinter),
}

impl ContentPrinter {
    fn printer_email(&self, content: &GetResult) -> anyhow::Result<String> {
        use std::fmt::Write;
        let mut output = String::new();

        writeln!(&mut output, "Subject: {}", content.title)?;

        writeln!(&mut output, "ID: {}", content.id,)?;

        writeln!(&mut output, "Version: {}", content.version.number,)?;

        writeln!(&mut output)?;

        writeln!(
            &mut output,
            "{}",
            html2text::from_read(content.body.storage.value.as_bytes(), 150)
        )?;

        Ok(output)
    }

    pub fn data_to_string(&self, content: &GetResult) -> anyhow::Result<String> {
        let r = match self {
            Self::Email => self.printer_email(content)?,
            Self::Serde(printer) => printer.data_to_string(content)?,
        };
        Ok(r)
    }
}

impl std::str::FromStr for ContentPrinter {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "email" => Ok(Self::Email),
            _ => Ok(Self::Serde(crate::printer::SerdePrinter::from_str(s)?)),
        }
    }
}
