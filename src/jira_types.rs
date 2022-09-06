use anyhow::Result;
use std::collections::HashMap;

use serde::Serialize;

#[derive(Serialize)]
pub struct IssueBeanFields {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee: Option<atlassian_jira_rest_types::v2::User>,
    pub attachment: Option<Vec<atlassian_jira_rest_types::v2::Attachment>>,
    pub comment: Option<atlassian_jira_rest_types::v2::PageOfComments>,
    pub components: Vec<atlassian_jira_rest_types::v2::ProjectComponent>,
    pub created: String,
    /// The details of the user created the issue.
    pub creator: atlassian_jira_rest_types::v2::User,
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issuelinks: Option<Vec<atlassian_jira_rest_types::v2::IssueLink>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issuetype: Option<atlassian_jira_rest_types::v2::IssueTypeDetails>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<atlassian_jira_rest_types::v2::Priority>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reporter: Option<atlassian_jira_rest_types::v2::User>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution: Option<atlassian_jira_rest_types::v2::Resolution>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolutiondate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<atlassian_jira_rest_types::v2::StatusDetails>,
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated: Option<String>,
    pub votes: atlassian_jira_rest_types::v2::Votes,
    pub watches: atlassian_jira_rest_types::v2::Watchers,
    pub custom_fields: HashMap<String, serde_json::Value>,
}

impl IssueBeanFields {
    pub fn of_json(value: std::collections::BTreeMap<String, serde_json::Value>) -> Result<Self> {
        Ok(Self {
            assignee: crate::serde::json_de_kv_opt(&value, "assignee")?,
            attachment: crate::serde::json_de_kv_opt(&value, "attachment")?,
            comment: crate::serde::json_de_kv_opt(&value, "comment")?,
            components: crate::serde::json_de_kv(&value, "components")?,
            created: crate::serde::json_de_kv(&value, "created")?,
            creator: crate::serde::json_de_kv(&value, "creator")?,
            description: crate::serde::json_de_kv(&value, "description")?,
            issuelinks: crate::serde::json_de_kv_opt(&value, "issuelinks")?,
            issuetype: crate::serde::json_de_kv_opt(&value, "issuetype")?,
            labels: crate::serde::json_de_kv_opt(&value, "labels")?,
            priority: crate::serde::json_de_kv_opt(&value, "priority")?,
            reporter: crate::serde::json_de_kv_opt(&value, "reporter")?,
            resolution: crate::serde::json_de_kv_opt(&value, "resolution")?,
            resolutiondate: crate::serde::json_de_kv_opt(&value, "resolutiondate")?,
            status: crate::serde::json_de_kv_opt(&value, "status")?,
            summary: crate::serde::json_de_kv(&value, "summary")?,
            updated: crate::serde::json_de_kv_opt(&value, "updated")?,
            votes: crate::serde::json_de_kv(&value, "votes")?,
            watches: crate::serde::json_de_kv(&value, "watches")?,
            custom_fields: value
                .into_iter()
                .filter(|(k, _)| k.starts_with("customfield_"))
                .collect(),
        })
    }
}

/// Details about an issue.
#[derive(Serialize)]
pub struct IssueBean {
    /// Details of changelogs associated with the issue.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub changelog: Option<atlassian_jira_rest_types::v2::PageOfChangelogs>,
    /// The metadata for the fields on the issue that can be amended.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub editmeta: Option<atlassian_jira_rest_types::v2::IssueUpdateMetadata>,
    /// Expand options that include additional issue details in the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expand: Option<String>,
    pub fields: IssueBeanFields,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "fieldsToInclude")]
    pub fields_to_include: Option<atlassian_jira_rest_types::v2::IncludedFields>,
    /// The ID of the issue.
    pub id: String,
    /// The key of the issue.
    pub key: String,
    /// The ID and name of each field present on the issue.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub names: Option<::std::collections::BTreeMap<String, String>>,
    /// The operations that can be performed on the issue.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operations: Option<atlassian_jira_rest_types::v2::Operations>,
    /// Details of the issue properties identified in the request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<::std::collections::BTreeMap<String, serde_json::Value>>,
    /// The rendered value of each field present on the issue.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "renderedFields")]
    pub rendered_fields: Option<::std::collections::BTreeMap<String, serde_json::Value>>,
    /// The schema describing each field present on the issue.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema:
        Option<::std::collections::BTreeMap<String, atlassian_jira_rest_types::v2::JsonTypeBean>>,
    /// The URL of the issue details.
    #[serde(rename = "self")]
    pub self_: String,
    /// The transitions that can be performed on the issue.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transitions: Option<Vec<atlassian_jira_rest_types::v2::IssueTransition>>,
    /// The versions of each field on the issue.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "versionedRepresentations")]
    pub versioned_representations: Option<
        ::std::collections::BTreeMap<
            String,
            ::std::collections::BTreeMap<String, serde_json::Value>,
        >,
    >,
}

impl IssueBean {
    pub fn of_json(value: atlassian_jira_rest_types::v2::IssueBean) -> Result<Self> {
        Ok(Self {
            changelog: value.changelog,
            editmeta: value.editmeta,
            expand: value.expand,
            fields: IssueBeanFields::of_json(value.fields)?,
            fields_to_include: value.fields_to_include,
            id: value.id,
            key: value.key,
            names: value.names,
            operations: value.operations,
            properties: value.properties,
            rendered_fields: value.rendered_fields,
            schema: value.schema,
            self_: value.self_,
            transitions: value.transitions,
            versioned_representations: value.versioned_representations,
        })
    }
}
