use anyhow::{Context, Result};
use serde::Deserialize;
use serde_yaml;
use std::{collections::HashMap, path::Path};

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Config {
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default)]
    pub crates: HashMap<String, CrateConfig>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct CrateConfig {
    pub labels: Vec<String>,
}

impl Config {
    pub async fn maybe_from_yaml(path: &dyn AsRef<Path>) -> Result<Option<Self>> {
        if let Err(e) = tokio::fs::metadata(path).await {
            return if e.kind() == std::io::ErrorKind::NotFound {
                Ok(None)
            } else {
                Err(e.into())
            };
        }

        let data = tokio::fs::read_to_string(path)
            .await
            .with_context(|| format!("failed to read config from {}", path.as_ref().display()))?;
        Ok(serde_yaml::from_str(&data)?)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            labels: vec!["rust".to_string(), "security".to_string()],
            crates: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn parse() {
        for (file, config) in [
            (
                "
labels:
  - foo
",
                Config {
                    labels: vec!["foo".to_string()],
                    crates: HashMap::new(),
                },
            ),
            (
                "
crates:
  foo:
    labels:
      - crate/foo
  bar:
    labels:
      - crate/bar
",
                Config {
                    labels: vec![],
                    crates: vec![
                        (
                            "foo".to_string(),
                            CrateConfig {
                                labels: vec!["crate/foo".to_string()],
                            },
                        ),
                        (
                            "bar".to_string(),
                            CrateConfig {
                                labels: vec!["crate/bar".to_string()],
                            },
                        ),
                    ]
                    .into_iter()
                    .collect(),
                },
            ),
        ]
        .into_iter()
        {
            assert_eq!(
                serde_yaml::from_str::<Config>(file)
                    .expect(&*format!("config must be ok:\n{}", file)),
                config
            );
        }
    }
}
