#![deny(warnings, rust_2018_idioms)]

pub mod client;
pub mod config;
pub mod deny;

pub use self::{client::Client, config::Config};

#[derive(Clone, Debug)]
pub struct GitHubRepo {
    pub owner: String,
    pub name: String,
}

#[derive(Clone, Debug)]
pub struct Advisory {
    pub title: String,
    pub id: String,
    pub body: String,
    pub withdrawn: bool,
    pub crate_name: Option<String>,
}

// === impl Advisory ===

impl From<self::deny::output::Diagnostic> for Advisory {
    fn from(d: self::deny::output::Diagnostic) -> Self {
        let crate_name = Self::find_progenitor(d.graphs);
        let title = if let Some(c) = &crate_name {
            format!("{}: [{}] {}", c, d.advisory.id, d.message)
        } else {
            format!("[{}] {}", d.advisory.id, d.message)
        };
        Self {
            title,
            id: d.advisory.id,
            body: d.advisory.description,
            withdrawn: d.advisory.withdrawn.is_some(),
            crate_name,
        }
    }
}

impl Advisory {
    fn find_progenitor(graphs: Vec<self::deny::output::Graph>) -> Option<String> {
        fn find(g: &self::deny::output::Graph) -> (usize, String) {
            g.parents
                .iter()
                .map(find)
                .max_by_key(|(d, _)| *d)
                .unwrap_or_else(|| (0, g.name.clone()))
        }

        graphs
            .iter()
            .map(find)
            .max_by_key(|(d, _)| *d)
            .map(|(_, n)| n)
    }
}

// === impl GitHubRepo ===

impl std::str::FromStr for GitHubRepo {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split('/');
        let owner = parts
            .next()
            .ok_or_else(|| anyhow::anyhow!("missing owner"))?;
        let name = parts
            .next()
            .ok_or_else(|| anyhow::anyhow!("missing repo"))?;
        if parts.next().is_some() {
            anyhow::bail!("too many parts");
        }
        Ok(Self {
            owner: owner.to_string(),
            name: name.to_string(),
        })
    }
}
