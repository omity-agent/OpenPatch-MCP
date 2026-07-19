use anyhow::Context as _;
use core::fmt;
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FileState {
    Missing,
    Present(String),
}
impl FileState {
    #[expect(
        clippy::pattern_type_mismatch,
        reason = "matching the borrowed enum keeps the returned contents borrowed"
    )]
    pub(super) const fn contents(&self) -> Option<&String> {
        match self {
            &Self::Missing => None,
            Self::Present(contents) => Some(contents),
        }
    }
    #[expect(
        clippy::pattern_type_mismatch,
        reason = "the state content is borrowed for the SQLite parameter"
    )]
    pub(super) fn database_parts(&self) -> (i64, Option<&str>) {
        match self {
            Self::Missing => (0, None),
            Self::Present(contents) => (1, Some(contents)),
        }
    }
    pub(super) fn from_database(
        present: i64,
        stored_contents: Option<String>,
    ) -> anyhow::Result<Self> {
        match (present, stored_contents) {
            (0, None) => Ok(Self::Missing),
            (1, Some(contents)) => Ok(Self::Present(contents)),
            _ => anyhow::bail!("invalid file state in operation history"),
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Mutation {
    pub(super) kind: OperationKind,
    pub(super) display_path: PathBuf,
    pub(super) changes: Vec<PathChange>,
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
            changes: vec![PathChange {
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
            changes: vec![
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
    pub(super) changes: Vec<PathChange>,
}
impl StoredOperation {
    pub(super) fn change(&self, role: PathRole) -> anyhow::Result<&PathChange> {
        self.changes
            .iter()
            .find(|change| change.role == role)
            .ok_or_else(|| anyhow::anyhow!("operation history is missing a {role:?} path"))
    }
}
