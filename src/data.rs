use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use chrono::NaiveDateTime;

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Deserialize)]
pub struct FcpWithInfo {
    pub fcp: FcpProposal,
    pub reviews: Vec<(GitHubUser, bool)>,
    pub issue: Issue,
    pub status_comment: IssueComment,
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Deserialize)]
pub struct FcpProposal {
    pub id: i32,
    pub fk_issue: i32,
    pub fk_initiator: i32,
    pub fk_initiating_comment: i32,
    pub disposition: String,
    pub fk_bot_tracking_comment: i32,
    pub fcp_start: Option<NaiveDateTime>,
    pub fcp_closed: bool,
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Deserialize)]
pub struct GitHubUser {
    pub id: i32,
    pub login: String,
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Deserialize)]
pub struct Issue {
    pub id: i32,
    pub number: i32,
    pub fk_milestone: Option<i32>,
    pub fk_user: i32,
    pub fk_assignee: Option<i32>,
    pub open: bool,
    pub is_pull_request: bool,
    pub title: String,
    pub body: String,
    pub locked: bool,
    pub closed_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub labels: Vec<String>,
    pub repository: String,
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Deserialize)]
pub struct IssueComment {
    pub id: i32,
    pub fk_issue: i32,
    pub fk_user: i32,
    pub body: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub repository: String,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FCPInfo {
    pub id: i32,
    pub tags: Vec<String>,
    pub title: String,

    pub repo: String,
    pub issue: i32,
    pub is_pr: bool,

    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,

    pub approved: Vec<String>,
    pub pending: Vec<String>,
}

impl From<FcpWithInfo> for FCPInfo {
    fn from(info: FcpWithInfo) -> Self {
        let mut approved = Vec::new();
        let mut pending = Vec::new();

        for (reviewer, approval) in info.reviews {
            if approval {
                approved.push(reviewer.login);
            } else {
                pending.push(reviewer.login);
            }
        }

        Self {
            id: info.fcp.id,

            tags: info.issue.labels,
            title: info.issue.title,

            repo: info.issue.repository,
            issue: info.issue.number,
            is_pr: info.issue.is_pull_request,

            created_at: info.issue.created_at,
            updated_at: info.issue.updated_at,

            approved, pending,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SentMsg {
    pub id: i64,
    pub version: NaiveDateTime,
    pub format: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FCPStorage {
    pub info: FCPInfo,
    pub messages: HashMap<String, SentMsg>,
}