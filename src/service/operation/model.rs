mod file;
use anyhow::Context as _;
use core::fmt;
pub(crate) use file::{FileContents, FileState};
use smallvec::{SmallVec, smallvec};
use std::path::PathBuf;
use uuid::Uuid;
use uuid_simd::UuidExt;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct OperationId(Uuid);
impl OperationId {
    pub(super) fn now_v7() -> Self {
        Self(Uuid::from_u128(fast_uuid_v7::gen_id()))
    }
    pub(super) fn parse(input: &str) -> anyhow::Result<Self> {
        <Uuid as UuidExt>::parse_hyphenated(input)
            .map(Self)
            .context("invalid UUID")
    }
    pub(super) fn from_slice(bytes: &[u8]) -> anyhow::Result<Self> {
        Uuid::from_slice(bytes)
            .map(Self)
            .context("invalid UUID in operation history")
    }
    pub(super) const fn as_bytes(&self) -> &[u8; 16] {
        self.0.as_bytes()
    }
}
impl fmt::Display for OperationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        UuidExt::format_hyphenated(&self.0).fmt(f)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OperationKind {
    Add,
    Edit,
    Delete,
}
impl OperationKind {
    pub(super) const fn tag(self) -> &'static str {
        match self {
            Self::Add => "ADD",
            Self::Edit => "EDIT",
            Self::Delete => "DELETE",
        }
    }
    pub(super) const fn code(self) -> i64 {
        match self {
            Self::Add => 0,
            Self::Edit => 1,
            Self::Delete => 2,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PathRole {
    Single,
    Source,
    Destination,
}
impl PathRole {
    pub(super) const fn code(self) -> i64 {
        match self {
            Self::Single => 0,
            Self::Source => 1,
            Self::Destination => 2,
        }
    }
    pub(super) fn from_code(code: i64) -> anyhow::Result<Self> {
        match code {
            0 => Ok(Self::Single),
            1 => Ok(Self::Source),
            2 => Ok(Self::Destination),
            _ => anyhow::bail!("invalid path role in history: {code}"),
        }
    }
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
