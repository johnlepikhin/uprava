use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct UpdateContentBodyVersion {
    pub number: u64,
}

#[derive(Serialize, Debug)]
pub struct UpdateContentBody {
    pub version: UpdateContentBodyVersion,
    pub title: String,
    #[serde(rename = "type")]
    pub content_type: crate::confluence_types::ContentType,
    pub body: crate::confluence_types::ContentBody,
}
