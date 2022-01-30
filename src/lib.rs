#![deny(warnings, rust_2018_idioms)]

pub mod client;
pub mod deny;

pub use self::client::Client;

#[derive(Clone, Debug)]
pub struct Advisory {
    pub progenitor: Option<String>,
    pub id: String,
    pub message: String,
    pub body: String,
}

impl From<self::deny::output::Diagnostic> for Advisory {
    fn from(d: self::deny::output::Diagnostic) -> Self {
        let progenitor = Self::find_progenitor(d.graphs);
        Self {
            progenitor,
            id: d.advisory.id,
            message: d.message,
            body: d.advisory.description,
        }
    }
}

impl Advisory {
    pub fn title(&self) -> String {
        if let Some(progenitor) = &self.progenitor {
            format!("{}: [{}] {}", progenitor, self.id, self.message)
        } else {
            format!("[{}] {}", self.id, self.message)
        }
    }

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
