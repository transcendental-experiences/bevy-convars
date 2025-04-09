use toml_edit::{ImDocument, Item, Table};

use crate::{reflect::ReflectCVar, CVarFlags, CVarManagement, CVarTreeNode};

pub(crate) type UnparsedCVar<'a> = (&'a str, Item);

pub(crate) struct CVarDocScanner<S: AsRef<str>> {
    document: ImDocument<S>,
    source: String,
}

/// A toml document and it's associated source data
pub struct DocumentContext<S: AsRef<str>> {
    document: ImDocument<S>,
    source: String,
}

impl<S: AsRef<str>> DocumentContext<S> {
    /// Creates a new DocumentContext.
    pub fn new(document: ImDocument<S>, source: String) -> Self {
        Self { document, source }
    }
}

impl<S: AsRef<str>> CVarDocScanner<S> {
    pub fn new(document: DocumentContext<S>) -> Self {
        Self {
            document: document.document,
            source: document.source,
        }
    }

    /// Recursively traverse a TOML document for CVars.
    fn traverse(&self, item: &Table, management: &CVarManagement, tree: &CVarTreeNode, outp: &mut Vec<UnparsedCVar<'_>>) {
        for (key, node) in tree.children().unwrap() {
            // Check if the node key exists within the document we're traversing, and if so get the value.
            println!("{key}");
            if let Some((_, value)) = item.get_key_value(key) {
                if node.is_leaf() {
                    let CVarTreeNode::Leaf { name, reg } = node else { unreachable!() };

                    let meta = management.resources[reg].data::<ReflectCVar>().unwrap();

                    if meta.flags().contains(CVarFlags::SAVED) {
                        outp.push((*name, value.clone()));
                    } else {
                        bevy_log::warn!("Found cvar {name} in {}, but that CVar cannot be saved (and as such cannot be loaded.)", self.source);
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

        self.traverse(self.document.as_table(),  management, &management.tree, &mut outp);

        outp
    }
}
