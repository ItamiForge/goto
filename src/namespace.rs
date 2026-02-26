use anyhow::{bail, Result};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use crate::config::{self, NamespaceDef};

#[derive(Debug)]
struct NamespaceEntry {
    name: String,
    path: PathBuf,
    aliases: Vec<String>,
}

#[derive(Debug)]
pub(crate) struct NamespaceMap {
    primary: BTreeMap<String, NamespaceEntry>,
}

impl NamespaceMap {
    pub(crate) fn load() -> Result<Self> {
        let parsed = config::load_config()?;
        Self::from_defs(parsed.namespace)
    }

    pub(crate) fn resolve(&self, target: &str) -> Result<PathBuf> {
        let (namespace, remainder) = match target.split_once('/') {
            Some((ns, rest)) => (ns, Some(rest)),
            None => (target, None),
        };

        let entry = self.lookup_namespace(namespace)?;
        let mut candidate = entry.path.clone();
        if let Some(rest) = remainder {
            candidate = candidate.join(rest);
        }

        if !candidate.exists() {
            bail!("path does not exist: {}", candidate.display());
        }

        Ok(candidate.canonicalize().unwrap_or(candidate))
    }

    pub(crate) fn list(&self, namespace: Option<&str>) -> Result<()> {
        if let Some(ns) = namespace {
            let entry = self.lookup_namespace(ns)?;
            self.list_dir(&entry.name, &entry.path)?;
            return Ok(());
        }

        for entry in self.primary.values() {
            self.list_dir(&entry.name, &entry.path)?;
        }

        Ok(())
    }

    pub(crate) fn complete(&self, partial: &str) -> Result<Vec<String>> {
        let trimmed = partial.trim();

        if trimmed.is_empty() {
            return Ok(self.complete_namespaces(""));
        }

        if let Some((namespace, rest)) = trimmed.split_once('/') {
            let entry = match self.lookup_namespace(namespace) {
                Ok(entry) => entry,
                Err(_) => return Ok(Vec::new()),
            };
            return Ok(self.complete_namespace_path(entry, namespace, rest));
        }

        Ok(self.complete_namespaces(trimmed))
    }

    pub(crate) fn from_defs(defs: Vec<NamespaceDef>) -> Result<Self> {
        if defs.is_empty() {
            bail!("config must contain at least one [[namespace]] entry");
        }

        let mut namespace_names = BTreeSet::new();
        for ns in &defs {
            let canonical_name = ns.name.trim().to_lowercase();
            if canonical_name.is_empty() {
                bail!("namespace names must not be empty");
            }
            if !namespace_names.insert(canonical_name.clone()) {
                bail!("duplicate namespace: {}", ns.name);
            }
        }

        let mut primary = BTreeMap::new();
        let mut global_aliases = BTreeSet::new();

        for ns in defs {
            let canonical_name = ns.name.trim().to_lowercase();

            let mut aliases = ns.aliases.unwrap_or_default();
            aliases.retain(|alias| !alias.trim().is_empty());
            aliases.sort_by_key(|alias| alias.to_lowercase());
            aliases.dedup_by(|left, right| left.eq_ignore_ascii_case(right));

            let mut normalized_aliases = Vec::new();
            for alias in aliases {
                let alias_key = alias.trim().to_lowercase();
                if alias_key.is_empty() {
                    continue;
                }
                if alias_key == canonical_name {
                    continue;
                }
                if namespace_names.contains(&alias_key) {
                    bail!("alias '{alias}' conflicts with namespace name");
                }
                if !global_aliases.insert(alias_key) {
                    bail!("duplicate alias across namespaces: {alias}");
                }
                normalized_aliases.push(alias);
            }

            let entry = NamespaceEntry {
                name: ns.name,
                path: config::expand_path(&ns.path),
                aliases: normalized_aliases,
            };

            primary.insert(canonical_name, entry);
        }

        Ok(Self { primary })
    }

    fn list_dir(&self, namespace: &str, root: &Path) -> Result<()> {
        let mut children = Vec::new();

        if root.exists() && root.is_dir() {
            for item in fs::read_dir(root)? {
                let entry = item?;
                if entry.file_type()?.is_dir() {
                    children.push(entry.file_name().to_string_lossy().into_owned());
                }
            }
            children.sort();
        }

        if children.is_empty() {
            println!("{namespace}");
            return Ok(());
        }

        for child in children {
            println!("{namespace}/{child}");
        }

        Ok(())
    }

    fn complete_namespaces(&self, prefix: &str) -> Vec<String> {
        let needle = prefix.to_lowercase();
        let mut entries = BTreeSet::new();

        for entry in self.primary.values() {
            for candidate in std::iter::once(&entry.name).chain(entry.aliases.iter()) {
                if needle.is_empty() || candidate.to_lowercase().starts_with(&needle) {
                    entries.insert(format!("{candidate}/"));
                }
            }
        }

        entries.into_iter().collect()
    }

    fn complete_namespace_path(
        &self,
        entry: &NamespaceEntry,
        namespace_input: &str,
        rest: &str,
    ) -> Vec<String> {
        let (dir_part, prefix) = if rest.is_empty() {
            (String::new(), String::new())
        } else if rest.ends_with('/') {
            (rest.trim_end_matches('/').to_string(), String::new())
        } else if let Some((parent, leaf)) = rest.rsplit_once('/') {
            (parent.to_string(), leaf.to_string())
        } else {
            (String::new(), rest.to_string())
        };

        let mut base = entry.path.clone();
        if !dir_part.is_empty() {
            base = base.join(&dir_part);
        }

        let mut entries = BTreeSet::new();
        if base.exists() && base.is_dir() {
            let needle = prefix.to_lowercase();
            if let Ok(read_dir) = fs::read_dir(&base) {
                for item in read_dir.flatten() {
                    if let Ok(file_type) = item.file_type() {
                        if !file_type.is_dir() {
                            continue;
                        }
                    }

                    let name = item.file_name().to_string_lossy().into_owned();
                    if !needle.is_empty() && !name.to_lowercase().starts_with(&needle) {
                        continue;
                    }

                    let mut candidate = format!("{namespace_input}/");
                    if !dir_part.is_empty() {
                        candidate.push_str(&dir_part);
                        candidate.push('/');
                    }
                    candidate.push_str(&name);
                    entries.insert(candidate);
                }
            }
        }

        entries.into_iter().collect()
    }

    fn lookup_namespace(&self, lookup: &str) -> Result<&NamespaceEntry> {
        let normalized = lookup.to_lowercase();

        if let Some(entry) = self.primary.get(&normalized) {
            return Ok(entry);
        }

        for entry in self.primary.values() {
            if entry
                .aliases
                .iter()
                .any(|alias| alias.eq_ignore_ascii_case(lookup))
            {
                return Ok(entry);
            }
        }

        bail!("unknown namespace: {lookup}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn resolves_alias_and_project() {
        let dir = tempdir().expect("create tempdir");
        fs::create_dir(dir.path().join("alpha")).expect("create child dir");

        let defs = vec![NamespaceDef {
            name: "gh".to_string(),
            path: dir.path().to_string_lossy().into_owned(),
            aliases: Some(vec!["github".to_string()]),
        }];

        let map = NamespaceMap::from_defs(defs).expect("build namespace map");
        let resolved = map.resolve("github/alpha").expect("resolve alias path");
        assert!(resolved.ends_with("alpha"));
    }

    #[test]
    fn resolve_missing_namespace_errors() {
        let defs = vec![NamespaceDef {
            name: "gh".to_string(),
            path: "/tmp".to_string(),
            aliases: None,
        }];

        let map = NamespaceMap::from_defs(defs).expect("build namespace map");
        let err = map
            .resolve("work")
            .expect_err("missing namespace should fail");
        assert!(err.to_string().contains("unknown namespace"));
    }

    #[test]
    fn rejects_duplicate_namespaces() {
        let defs = vec![
            NamespaceDef {
                name: "gh".to_string(),
                path: "/tmp".to_string(),
                aliases: None,
            },
            NamespaceDef {
                name: "GH".to_string(),
                path: "/tmp".to_string(),
                aliases: None,
            },
        ];

        let err = NamespaceMap::from_defs(defs).expect_err("duplicate namespaces must fail");
        assert!(err.to_string().contains("duplicate namespace"));
    }
}
