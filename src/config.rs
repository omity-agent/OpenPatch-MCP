use serde::Deserialize;
use std::path::{Path, PathBuf};
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct Settings {
    #[serde(default)]
    pub executable_path: Option<PathBuf>,
    #[serde(default)]
    pub arguments: Vec<String>,
}
impl Settings {
    pub async fn read(path: Option<&Path>) -> anyhow::Result<Self> {
        let Some(settings_path) = path.or_else(|| {
            Path::new("settings.toml")
                .exists()
                .then_some(Path::new("settings.toml"))
        }) else {
            return Ok(Self::default());
        };
        let content = tokio::fs::read_to_string(settings_path).await?;
        let settings: Self = toml::from_str(&content)?;
        Ok(settings.normalized())
    }
    #[must_use]
    pub fn normalized(mut self) -> Self {
        if self
            .executable_path
            .as_ref()
            .is_some_and(|path| path.as_os_str().is_empty())
        {
            self.executable_path = None;
        }
        self.arguments.retain(|argument| !argument.is_empty());
        self
    }
}
#[cfg(test)]
#[expect(
    clippy::inline_modules,
    reason = "unit tests stay next to private helpers"
)]
mod tests {
    use super::Settings;
    #[test]
    fn normalized_removes_empty_path_and_arguments() {
        let settings = Settings {
            executable_path: Some("".into()),
            arguments: vec![String::new(), String::from("run")],
        };
        assert_eq!(
            settings.normalized(),
            Settings {
                executable_path: None,
                arguments: vec![String::from("run")]
            }
        );
    }
}
