use anyhow::Result;
use std::collections::{HashMap, HashSet};

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
pub struct IssueID {
    pub jira: crate::jira::JiraServer,
    pub issue: String,
}

impl IssueID {
    pub fn of_issue(issue: &crate::report::ReportIssue) -> Self {
        Self {
            jira: issue.jira.clone(),
            issue: issue.issue.key.clone(),
        }
    }

    pub fn of_relation_subject(val: &crate::report::ForeignRelationSubject) -> Self {
        Self {
            jira: val.jira.clone(),
            issue: val.issue.clone(),
        }
    }

    pub fn new(jira: &crate::jira::JiraServer, key: &str) -> Self {
        Self {
            jira: jira.clone(),
            issue: key.to_owned(),
        }
    }

    pub fn as_string(&self) -> String {
        format!("{}/{}", self.jira.base_url, self.issue)
            .replace(':', "_")
            .replace('/', "_")
            .replace('-', "_")
            .replace('.', "_")
    }
}

#[derive(Clone)]
pub struct IssuesList {
    pub issues: HashMap<IssueID, crate::report::ReportIssue>,
}

impl IssuesList {
    pub fn new() -> Self {
        Self {
            issues: HashMap::new(),
        }
    }

    pub fn of_slice(slice: &[crate::report::ReportIssue]) -> Self {
        let issues = slice
            .iter()
            .map(|issue| (IssueID::of_issue(issue), issue.clone()))
            .collect();
        Self { issues }
    }

    pub fn insert(&mut self, issue: &crate::report::ReportIssue) {
        let _ = self.issues.insert(IssueID::of_issue(issue), issue.clone());
    }

    pub fn get(
        &self,
        jira: &crate::jira::JiraServer,
        key: &str,
    ) -> Option<&crate::report::ReportIssue> {
        let id = IssueID::new(jira, key);
        self.issues.get(&id)
    }

    pub async fn get_fetch(
        &mut self,
        jira: &crate::jira::JiraServer,
        key: &str,
        entity_type: crate::report::ReportIssueType,
    ) -> Result<&crate::report::ReportIssue> {
        let id = IssueID::new(jira, key);
        if self.issues.contains_key(&id) {
            Ok(self.issues.get(&id).unwrap())
        } else {
            let issue = crate::report::ReportIssue::of_issuebean(
                jira,
                &jira.issue_bean(key).await?,
                entity_type,
            )?;
            let _ = self.issues.insert(IssueID::of_issue(&issue), issue);
            Ok(self.issues.get(&id).unwrap())
        }
    }

    pub fn all(&self) -> &HashMap<IssueID, crate::report::ReportIssue> {
        &self.issues
    }
}

#[derive(Hash, PartialEq, Eq)]
pub struct Relation {
    pub from: IssueID,
    pub to: IssueID,
    pub kind: crate::report::RelationKind,
}

pub struct ReportData {
    pub issues: IssuesList,
    pub epics: IssuesList,
    pub relations: HashSet<Relation>,
}

impl ReportData {
    fn issue_link(
        issue: &crate::report::ReportIssue,
        link: &atlassian_jira_rest_types::v2::IssueLink,
    ) -> Option<(crate::jira::JiraServer, String, String, bool)> {
        match &link.inward_issue {
            None => {
                let outward_issue = match &link.outward_issue {
                    None => return None,
                    Some(v) => v,
                };
                let key = match &outward_issue.key {
                    None => return None,
                    Some(v) => v,
                };
                let kind = match &link.type_.inward {
                    None => return None,
                    Some(v) => v,
                };
                let kind = match issue
                    .jira
                    .relations_map
                    .iter()
                    .filter(|(k, _)| k == kind)
                    .collect::<Vec<_>>()
                    .first()
                {
                    Some((_, v)) => v,
                    None => kind,
                };
                Some((issue.jira.clone(), key.to_owned(), kind.to_owned(), true))
            }
            Some(inward_issue) => {
                let key = match &inward_issue.key {
                    None => return None,
                    Some(v) => v,
                };
                let kind = match &link.type_.inward {
                    None => return None,
                    Some(v) => v,
                };
                let kind = match issue
                    .jira
                    .relations_map
                    .iter()
                    .filter(|(k, _)| k == kind)
                    .collect::<Vec<_>>()
                    .first()
                {
                    Some((_, v)) => v,
                    None => kind,
                };
                Some((issue.jira.clone(), key.to_owned(), kind.to_owned(), false))
            }
        }
    }

    fn issue_links(
        foreign_relations: &[crate::report::ForeignRelation],
        issue: &crate::report::ReportIssue,
    ) -> Vec<(crate::jira::JiraServer, String, String, bool)> {
        let mut r: Vec<_> = issue
            .issue
            .fields
            .issuelinks
            .as_deref()
            .unwrap_or_default()
            .iter()
            .filter_map(|link| Self::issue_link(issue, link))
            .collect();

        let issue_id = IssueID::of_issue(issue);

        for relation in foreign_relations {
            let from_id = IssueID::of_relation_subject(&relation.from);
            let to_id = IssueID::of_relation_subject(&relation.to);
            if from_id == issue_id {
                r.push((
                    to_id.jira.clone(),
                    relation.to.issue.clone(),
                    relation.to.kind.clone(),
                    false,
                ))
            }
            if to_id == issue_id {
                r.push((
                    from_id.jira.clone(),
                    relation.from.issue.clone(),
                    relation.from.kind.clone(),
                    false,
                ))
            }
        }
        r
    }

    async fn get_relations(
        foreign_relations: &[crate::report::ForeignRelation],
        issues: &mut IssuesList,
        deepness: usize,
        ignore_fetch_errors: bool,
    ) -> Result<HashSet<Relation>> {
        let mut relations = HashSet::new();

        slog_scope::info!("Fetching relations for issues list");

        let mut issues_to_process: Vec<_> = issues.all().values().cloned().collect();
        for deepness_level in 0..deepness {
            slog_scope::info!("Fetching relations at level {}", deepness_level + 1);
            let mut new_issues_to_process = HashMap::new();
            for issue in &issues_to_process {
                slog_scope::info!("Fetching relations for {:?}", issue.issue.key);
                let issue_links = Self::issue_links(foreign_relations, issue);
                slog_scope::debug!("Calculated and found {} links", issue_links.len());
                for (link_jira, link_key, kind, reversed) in issue_links {
                    let linked_issue = issues.get(&link_jira, &link_key);
                    let issue_registered = linked_issue.is_some();
                    let (issue1, issue2) = if reversed {
                        (
                            IssueID::new(&link_jira, &link_key),
                            IssueID::of_issue(issue),
                        )
                    } else {
                        (
                            IssueID::of_issue(issue),
                            IssueID::new(&link_jira, &link_key),
                        )
                    };
                    let relation_added = match kind.as_str() {
                        "dependance for" => {
                            let _ = relations.insert(Relation {
                                from: issue1,
                                to: issue2,
                                kind: crate::report::RelationKind::Dependance,
                            });
                            true
                        }
                        "depends on" => {
                            let _ = relations.insert(Relation {
                                from: issue2,
                                to: issue1,
                                kind: crate::report::RelationKind::Dependance,
                            });
                            true
                        }
                        "mentioned in" | "relates to" => {
                            let _ = relations.insert(Relation {
                                from: issue1,
                                to: issue2,
                                kind: crate::report::RelationKind::Mention,
                            });
                            true
                        }
                        "mentions" => {
                            let _ = relations.insert(Relation {
                                from: issue2,
                                to: issue1,
                                kind: crate::report::RelationKind::Mention,
                            });
                            true
                        }
                        "blocks" => {
                            let _ = relations.insert(Relation {
                                from: issue1,
                                to: issue2,
                                kind: crate::report::RelationKind::Block,
                            });
                            true
                        }
                        "is blocked by" => {
                            let _ = relations.insert(Relation {
                                from: issue2,
                                to: issue1,
                                kind: crate::report::RelationKind::Block,
                            });
                            true
                        }
                        _ => {
                            slog_scope::error!(
                                "Unknown relation kind {:?} in {:?} or {:?}",
                                kind,
                                issue.issue.key,
                                link_key
                            );
                            false
                        }
                    };
                    if relation_added && !issue_registered {
                        match linked_issue {
                            None => {
                                let result = issues
                                    .get_fetch(
                                        &link_jira,
                                        &link_key,
                                        crate::report::ReportIssueType::ExternalDependency,
                                    )
                                    .await;
                                match result {
                                    Ok(linked_issue) => {
                                        let _ = new_issues_to_process.insert(
                                            IssueID::of_issue(linked_issue),
                                            linked_issue.clone(),
                                        );
                                    }
                                    Err(err) => {
                                        if !ignore_fetch_errors {
                                            return Err(err);
                                        } else {
                                            slog_scope::warn!("Failed to fetch dependency, but ignore_fetch_errors is in action")
                                        }
                                    }
                                }
                            }
                            Some(linked_issue) => {
                                let _ = new_issues_to_process
                                    .insert(IssueID::of_issue(linked_issue), linked_issue.clone());
                            }
                        }
                    }
                }
            }
            issues_to_process = new_issues_to_process.into_values().collect()
        }
        Ok(relations)
    }

    async fn get_epics(issues: &mut IssuesList, ignore_fetch_errors: bool) -> Result<IssuesList> {
        slog_scope::info!("Fetching EPICs for issues list");

        let mut epics = IssuesList::new();
        let issues_to_process: Vec<_> = issues.all().values().cloned().collect();
        for issue in &issues_to_process {
            let epic_key = match &issue.custom_fields.epic_link {
                None => continue,
                Some(v) => v,
            };
            if let Some(epic) = epics.get(&issue.jira, epic_key) {
                issues.insert(epic);
                continue;
            }
            match issues.get(&issue.jira, epic_key) {
                Some(v) => epics.insert(v),
                None => {
                    match epics
                        .get_fetch(&issue.jira, epic_key, crate::report::ReportIssueType::Epic)
                        .await
                    {
                        Ok(epic) => issues.insert(epic),
                        Err(err) => {
                            if !ignore_fetch_errors {
                                return Err(err);
                            } else {
                                slog_scope::warn!(
                                    "Failed to fetch EPIC, but ignore_fetch_errors is in action"
                                )
                            }
                        }
                    }
                }
            }
        }

        Ok(epics)
    }
    pub async fn of_slice(
        foreign_relations: &[crate::report::ForeignRelation],
        slice: &[crate::report::ReportIssue],
        dependencies_deepness: usize,
        ignore_fetch_errors: bool,
    ) -> Result<Self> {
        let mut issues = IssuesList::of_slice(slice);
        let relations = Self::get_relations(
            foreign_relations,
            &mut issues,
            dependencies_deepness,
            ignore_fetch_errors,
        )
        .await?;
        let epics = Self::get_epics(&mut issues, ignore_fetch_errors).await?;
        Ok(Self {
            issues,
            epics,
            relations,
        })
    }
}
