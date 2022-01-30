#![allow(clippy::large_enum_variant)]

use super::Advisory;
use anyhow::Result;
use std::path::PathBuf;

/// Lists all RUSTSEC advisories for a given crate/workspace.
pub async fn advisories(cargo_deny_path: PathBuf, target_dir: PathBuf) -> Result<Vec<Advisory>> {
    let std::process::Output {
        stderr,
        status: _,
        stdout: _,
    } = tokio::process::Command::new(cargo_deny_path)
        .args(vec!["--format=json", "check", "advisories"])
        .current_dir(target_dir)
        .output()
        .await?;

    serde_json::Deserializer::from_slice(&*stderr)
        .into_iter::<output::Object>()
        .map(|r| r.map_err(anyhow::Error::from))
        .filter_map(|d| match d {
            Ok(output::Object::Diagnostic(d)) => Some(Ok(Advisory::from(d))),
            Ok(_) => None,
            Err(e) => Some(Err(e)),
        })
        .collect()
}

pub(crate) mod output {
    use serde::Deserialize;

    #[derive(Clone, Debug, Deserialize)]
    #[serde(tag = "type", content = "fields", rename_all = "lowercase")]
    pub enum Object {
        Diagnostic(Diagnostic),
        Summary(Summary),
    }

    #[derive(Clone, Debug, Deserialize)]
    pub struct Summary {
        pub advisories: AdvisorySummary,
    }

    #[derive(Clone, Debug, Deserialize)]
    pub struct AdvisorySummary {
        pub errors: usize,
        pub helps: usize,
        pub notes: usize,
        pub warnings: usize,
    }

    #[derive(Clone, Debug, Deserialize)]
    pub struct Diagnostic {
        pub advisory: Advisory,
        pub code: String,
        pub message: String,
        pub graphs: Vec<Graph>,
        pub labels: Vec<Label>,
        pub notes: Vec<String>,
        pub severity: String,
    }

    #[derive(Clone, Debug, Deserialize)]
    pub struct Advisory {
        pub aliases: Vec<String>,
        pub categories: Vec<String>,
        pub collection: String,
        pub cvss: Option<String>,
        pub date: String,
        pub description: String,
        pub id: String,
        pub keywords: Vec<String>,
        pub package: String,
        pub references: Vec<String>,
        pub related: Vec<String>,
        pub title: String,
        pub url: String,
        pub withdrawn: Option<String>,
    }

    #[derive(Clone, Debug, Deserialize)]
    pub struct Graph {
        pub name: String,
        #[serde(default)]
        pub parents: Vec<Graph>,
        #[serde(default)]
        pub repeat: bool,
        pub version: String,
    }

    #[derive(Clone, Debug, Deserialize)]
    pub struct Label {
        pub column: usize,
        pub line: usize,
        pub message: String,
        pub span: String,
    }
}
