#![deny(warnings, rust_2018_idioms)]

use std::convert::Infallible;

pub mod client;
pub mod deny;

pub use self::client::Client;

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

#[derive(Clone, Debug)]
pub struct Labels(Vec<String>);

// === impl Advisory ===

impl TryFrom<self::deny::output::Diagnostic> for Advisory {
    type Error = self::deny::output::Diagnostic;

    fn try_from(d: self::deny::output::Diagnostic) -> Result<Self, Self::Error> {
        let a = match d.advisory {
            Some(a) => a,
            None => return Err(d),
        };
        let crate_name = d.graphs.and_then(Self::find_progenitor);
        let title = if let Some(c) = &crate_name {
            format!("{}: [{}] {}", c, a.id, d.message)
        } else {
            format!("[{}] {}", a.id, d.message)
        };
        Ok(Self {
            title,
            id: a.id,
            body: a.description,
            withdrawn: a.withdrawn.is_some(),
            crate_name,
        })
    }
}

impl Advisory {
    fn find_progenitor(graphs: Vec<self::deny::output::Graph>) -> Option<String> {
        fn find(g: &self::deny::output::Graph) -> (usize, String) {
            g.parents
                .iter()
                .map(|p| {
                    let (d, n) = find(p);
                    (d + 1, n)
                })
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

#[cfg(test)]
#[test]
fn test_find_progenitor() {
    use self::deny::output::Graph;

    let baz = Graph {
        name: "baz".to_string(),
        parents: vec![],
        repeat: false,
        version: "0.1.0".into(),
    };
    let bar = Graph {
        name: "bar".to_string(),
        parents: vec![baz],
        repeat: false,
        version: "0.1.0".into(),
    };
    let bah = Graph {
        name: "bah".to_string(),
        parents: vec![],
        repeat: false,
        version: "0.1.0".into(),
    };
    let foo = Graph {
        name: "foo".to_string(),
        parents: vec![bar, bah],
        repeat: false,
        version: "0.1.0".into(),
    };

    assert_eq!(
        Some("baz".to_string()),
        Advisory::find_progenitor(vec![foo])
    );
    assert_eq!(None, Advisory::find_progenitor(vec![]));
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

// === impl Labels ===

impl std::str::FromStr for Labels {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Infallible> {
        let labels = s
            .split(',')
            .map(|l| l.trim())
            .filter_map(|l| {
                if l.is_empty() {
                    None
                } else {
                    Some(l.to_string())
                }
            })
            .collect();
        Ok(Labels(labels))
    }
}
