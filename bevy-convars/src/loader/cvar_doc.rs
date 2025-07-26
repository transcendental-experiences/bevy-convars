use toml_edit::{ImDocument, Item, Table};

use crate::{CVarFlags, CVarManagement, CVarTreeNode, reflect::ReflectCVar};

pub(crate) type UnparsedCVar<'a> = (&'a str, Item);

pub(crate) struct CVarDocScanner<S: AsRef<str>> {
    document: ImDocument<S>,
    source: String,
    user_config: bool,
}

/// A toml document and it's associated source data
#[derive(Clone)]
pub struct DocumentContext<S: AsRef<str>> {
    document: ImDocument<S>,
    source: String,
}

impl Default for DocumentContext<String> {
    fn default() -> Self {
        Self {
            document: ImDocument::parse(String::new()).unwrap(),
            source: Default::default(),
        }
    }
}

impl<S: AsRef<str>> DocumentContext<S> {
    /// Creates a new DocumentContext.
    pub fn new(document: ImDocument<S>, source: String) -> Self {
        Self { document, source }
    }

    /// Returns the source of this document.
    pub fn source(&self) -> &str {
        &self.source
    }
}

impl<S: AsRef<str>> CVarDocScanner<S> {
    pub fn new(document: DocumentContext<S>, user_config: bool) -> Self {
        Self {
            document: document.document,
            source: document.source,
            user_config: user_config,
        }
    }

    /// Recursively traverse a TOML document for CVars.
    fn traverse(
        &self,
        item: &Table,
        management: &CVarManagement,
        tree: &CVarTreeNode,
        outp: &mut Vec<UnparsedCVar<'_>>,
    ) {
        for (key, node) in tree.children().unwrap() {
            // Check if the node key exists within the document we're traversing, and if so get the value.
            if let Some((_, value)) = item.get_key_value(key) {
                if node.is_leaf() {
                    let CVarTreeNode::Leaf { name, reg } = node else {
                        unreachable!()
                    };

                    let meta = management.resources[reg].data::<ReflectCVar>().unwrap();

                    if meta.flags().contains(CVarFlags::SAVED) || !self.user_config {
                        outp.push((*name, value.clone()));
                    } else {
                        bevy_log::warn!(
                            "Found cvar {name} in {}, but that CVar cannot be saved (and as such cannot be loaded.)",
                            self.source
                        );
                    }
                } else if let Some(item) = value.as_table() {
                    self.traverse(item, management, node, outp);
                } else {
                    bevy_log::warn!(
                        "When parsing {}, found a cvar-like key {key} that was expected to be a table. Was of type {}",
                        self.source,
                        value.type_name()
                    );
                }
            }
        }
    }

    pub fn find_cvars(&self, management: &CVarManagement) -> Vec<UnparsedCVar<'_>> {
        let mut outp = vec![];

        self.traverse(
            self.document.as_table(),
            management,
            &management.tree,
            &mut outp,
        );

        outp
    }
}
