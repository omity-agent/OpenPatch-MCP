pub(crate) use super::file_state::{FileContents, FileState};
use anyhow::Context as _;
use core::fmt;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use rmcp::schemars::JsonSchema;
use smallvec::{SmallVec, smallvec};
use sonic_rs::Serialize;
use std::path::PathBuf;
pub(crate) const ID_PATTERN: &str = "^oppc-[0-9a-z]{8}$";
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OperationId(i64);
impl OperationId {
    pub(super) const LIMIT: i64 = 36_i64.pow(8);
    pub(super) fn random() -> Self {
        Self(rand::random_range(0..Self::LIMIT))
    }
    pub(crate) fn parse(input: &str) -> anyhow::Result<Self> {
        const INVALID_ID: &str =
            "invalid operation ID: expected oppc- followed by 8 lowercase Base36 digits";
        let suffix = input.strip_prefix("oppc-").context(INVALID_ID)?;
        anyhow::ensure!(
            suffix.len() == 8
                && suffix
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || byte.is_ascii_lowercase()),
            INVALID_ID
        );
        Ok(Self(i64::from_str_radix(suffix, 36).context(INVALID_ID)?))
    }
    pub(super) fn from_stored(value: i64) -> anyhow::Result<Self> {
        anyhow::ensure!(
            (0..Self::LIMIT).contains(&value),
            "invalid operation ID in operation history: {value}"
        );
        Ok(Self(value))
    }
    pub(super) const fn value(self) -> i64 {
        self.0
    }
}
impl fmt::Display for OperationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let digits = radix_fmt::radix_36(self.0).to_string();
        write!(f, "oppc-{digits:0>8}")
    }
}
#[cfg(test)]
#[path = "../../../test/identifiers.rs"]
mod identifier_tests;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema, IntoPrimitive)]
#[repr(i64)]
#[serde(rename_all = "UPPERCASE")]
#[schemars(rename = "OutputOperationKind", rename_all = "UPPERCASE")]
pub(crate) enum OperationKind {
    Add = 0,
    Edit = 1,
    Delete = 2,
}
impl OperationKind {
    pub(super) const fn tag(self) -> &'static str {
        match self {
            Self::Add => "ADD",
            Self::Edit => "EDIT",
            Self::Delete => "DELETE",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoPrimitive, TryFromPrimitive)]
#[repr(i64)]
pub(crate) enum PathRole {
    Single = 0,
    Source = 1,
    Destination = 2,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PathChange {
    pub(super) role: PathRole,
    pub(super) path: PathBuf,
    pub(super) before: FileState,
    pub(super) after: FileState,
}
pub(super) type PathChanges = SmallVec<[PathChange; 2]>;
pub(super) type FileStates = SmallVec<[FileState; 2]>;
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Mutation {
    pub(super) kind: OperationKind,
    pub(super) display_path: PathBuf,
    pub(super) changes: PathChanges,
}
impl Mutation {
    pub(crate) fn single(
        kind: OperationKind,
        path: PathBuf,
        before: FileState,
        after: FileState,
    ) -> Self {
        Self {
            kind,
            display_path: path.clone(),
            changes: smallvec![PathChange {
                role: PathRole::Single,
                path,
                before,
                after,
            }],
        }
    }
    pub(crate) fn moved(
        source: PathBuf,
        destination: PathBuf,
        source_before: FileState,
        destination_before: FileState,
        destination_after: FileState,
    ) -> Self {
        Self {
            kind: OperationKind::Edit,
            display_path: destination.clone(),
            changes: smallvec![
                PathChange {
                    role: PathRole::Destination,
                    path: destination,
                    before: destination_before,
                    after: destination_after,
                },
                PathChange {
                    role: PathRole::Source,
                    path: source,
                    before: source_before,
                    after: FileState::Missing,
                },
            ],
        }
    }
    pub(super) fn change(&self, role: PathRole) -> anyhow::Result<&PathChange> {
        self.changes
            .iter()
            .find(|change| change.role == role)
            .ok_or_else(|| anyhow::anyhow!("operation history is missing a {role:?} path"))
    }
}
#[derive(Debug, Clone)]
pub(super) struct StoredOperation {
    pub(super) changes: PathChanges,
}
impl StoredOperation {
    pub(super) fn change(&self, role: PathRole) -> anyhow::Result<&PathChange> {
        self.changes
            .iter()
            .find(|change| change.role == role)
            .ok_or_else(|| anyhow::anyhow!("operation history is missing a {role:?} path"))
    }
}
