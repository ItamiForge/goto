use anyhow::{bail, Result};
use clap::Parser;
use std::collections::BTreeSet;

mod cli;
mod config;
mod namespace;
mod shell;

fn main() -> Result<()> {
    let cli = cli::Goto::parse();
    let namespaces = namespace::NamespaceMap::load()?;

    match cli.command {
        Some(cli::Command::CompleteTargets { partial }) => {
            for entry in namespaces.complete(partial.as_deref().unwrap_or(""))? {
                println!("{entry}");
            }
            Ok(())
        }
        Some(cli::Command::List { namespace }) => namespaces.list(namespace.as_deref()),
        Some(cli::Command::Setup) => shell::setup_zsh(),
        Some(cli::Command::Uninstall) => shell::uninstall_zsh(),
        Some(cli::Command::ConfigPath) => {
            println!("{}", config::config_file().display());
            Ok(())
        }
        Some(cli::Command::Doctor) => run_doctor(),
        Some(cli::Command::Add {
            name,
            path,
            aliases,
        }) => add_namespace(&name, &path, aliases),
        Some(cli::Command::Remove { name }) => remove_namespace(&name),
        Some(cli::Command::Rename { old, new }) => rename_namespace(&old, &new),
        Some(cli::Command::SetPath { name, path }) => set_namespace_path(&name, &path),
        Some(cli::Command::AliasAdd { namespace, alias }) => add_alias(&namespace, &alias),
        Some(cli::Command::AliasRemove { namespace, alias }) => remove_alias(&namespace, &alias),
        Some(cli::Command::ListRaw) => {
            let cfg = config::load_config()?;
            let rendered = config::render_config(&cfg)?;
            println!("{rendered}");
            Ok(())
        }
        Some(cli::Command::External(args)) => {
            let target = match args.as_slice() {
                [single] => single,
                [] => {
                    cli::print_help_with_setup_hint();
                    return Ok(());
                }
                _ => bail!("expected a single target like <namespace> or <namespace>/<path>"),
            };

            let path = namespaces.resolve(target)?;
            println!("{}", path.display());
            Ok(())
        }
        None => {
            cli::print_help_with_setup_hint();
            Ok(())
        }
    }
}

fn add_namespace(name: &str, path: &str, aliases: Vec<String>) -> Result<()> {
    let mut cfg = config::load_config_for_update()?;
    if find_namespace_index(&cfg, name).is_some() {
        bail!("namespace already exists: {name}");
    }

    let namespace = config::NamespaceDef {
        name: name.to_string(),
        path: path.to_string(),
        aliases: if aliases.is_empty() {
            None
        } else {
            Some(aliases)
        },
    };

    cfg.namespace.push(namespace);
    cfg.namespace.sort_by_key(|entry| entry.name.to_lowercase());
    namespace::NamespaceMap::from_defs(cfg.namespace.clone())?;
    config::save_config(&cfg)?;
    println!("added namespace '{name}'");
    Ok(())
}

fn remove_namespace(name: &str) -> Result<()> {
    let mut cfg = config::load_config_for_update()?;
    let Some(index) = find_namespace_or_alias_index(&cfg, name) else {
        bail!("unknown namespace or alias: {name}");
    };

    let removed = cfg.namespace.remove(index);
    if cfg.namespace.is_empty() {
        bail!("cannot remove last namespace; config must contain at least one");
    }

    namespace::NamespaceMap::from_defs(cfg.namespace.clone())?;
    config::save_config(&cfg)?;
    println!("removed namespace '{}'", removed.name);
    Ok(())
}

fn rename_namespace(old: &str, new: &str) -> Result<()> {
    if old.eq_ignore_ascii_case(new) {
        println!("namespace already named '{new}'; nothing changed");
        return Ok(());
    }

    let mut cfg = config::load_config_for_update()?;
    let Some(index) = find_namespace_index(&cfg, old) else {
        bail!("unknown namespace: {old}");
    };

    if find_namespace_index(&cfg, new).is_some() {
        bail!("namespace already exists: {new}");
    }

    let previous = cfg.namespace[index].name.clone();
    cfg.namespace[index].name = new.to_string();
    cfg.namespace.sort_by_key(|entry| entry.name.to_lowercase());

    namespace::NamespaceMap::from_defs(cfg.namespace.clone())?;
    config::save_config(&cfg)?;
    println!("renamed namespace '{previous}' -> '{new}'");
    Ok(())
}

fn set_namespace_path(name: &str, path: &str) -> Result<()> {
    let mut cfg = config::load_config_for_update()?;
    let Some(index) = find_namespace_index(&cfg, name) else {
        bail!("unknown namespace: {name}");
    };

    cfg.namespace[index].path = path.to_string();
    namespace::NamespaceMap::from_defs(cfg.namespace.clone())?;
    config::save_config(&cfg)?;
    println!("updated path for '{}'", cfg.namespace[index].name);
    Ok(())
}

fn add_alias(namespace_name: &str, alias: &str) -> Result<()> {
    let alias = alias.trim();
    if alias.is_empty() {
        bail!("alias must not be empty");
    }

    let mut cfg = config::load_config_for_update()?;
    let Some(index) = find_namespace_index(&cfg, namespace_name) else {
        bail!("unknown namespace: {namespace_name}");
    };

    let aliases = cfg.namespace[index].aliases.get_or_insert_with(Vec::new);
    if aliases
        .iter()
        .any(|value| value.eq_ignore_ascii_case(alias))
    {
        println!(
            "alias '{alias}' already present for '{}'; nothing changed",
            cfg.namespace[index].name
        );
        return Ok(());
    }
    aliases.push(alias.to_string());

    namespace::NamespaceMap::from_defs(cfg.namespace.clone())?;
    config::save_config(&cfg)?;
    println!("added alias '{alias}' to '{}'", cfg.namespace[index].name);
    Ok(())
}

fn remove_alias(namespace_name: &str, alias: &str) -> Result<()> {
    let mut cfg = config::load_config_for_update()?;
    let Some(index) = find_namespace_index(&cfg, namespace_name) else {
        bail!("unknown namespace: {namespace_name}");
    };

    let Some(aliases) = cfg.namespace[index].aliases.as_mut() else {
        bail!("namespace '{}' has no aliases", cfg.namespace[index].name);
    };

    let before = aliases.len();
    aliases.retain(|existing| !existing.eq_ignore_ascii_case(alias));
    if aliases.len() == before {
        bail!("alias not found: {alias}");
    }
    if aliases.is_empty() {
        cfg.namespace[index].aliases = None;
    }

    namespace::NamespaceMap::from_defs(cfg.namespace.clone())?;
    config::save_config(&cfg)?;
    println!(
        "removed alias '{alias}' from '{}'",
        cfg.namespace[index].name
    );
    Ok(())
}

fn run_doctor() -> Result<()> {
    let mut issues = Vec::new();

    let config_path = config::config_file();
    println!("[ok] config path: {}", config_path.display());

    let cfg = match config::load_config() {
        Ok(cfg) => {
            println!("[ok] config parses as TOML");
            cfg
        }
        Err(error) => {
            issues.push(format!("config parse/load failed: {error}"));
            report_doctor(issues);
            bail!("doctor found issues");
        }
    };

    match namespace::NamespaceMap::from_defs(cfg.namespace.clone()) {
        Ok(_) => println!("[ok] namespace validation passed"),
        Err(error) => issues.push(format!("namespace validation failed: {error}")),
    }

    let mut seen_aliases = BTreeSet::new();
    for ns in &cfg.namespace {
        let expanded = config::expand_path(&ns.path);
        if expanded.exists() && expanded.is_dir() {
            println!(
                "[ok] namespace '{}' root exists: {}",
                ns.name,
                expanded.display()
            );
        } else {
            issues.push(format!(
                "namespace '{}' root missing or not a directory: {}",
                ns.name,
                expanded.display()
            ));
        }

        if let Some(aliases) = &ns.aliases {
            for alias in aliases {
                let key = alias.to_lowercase();
                if !seen_aliases.insert(key) {
                    issues.push(format!("duplicate alias found: {alias}"));
                }
            }
        }
    }

    if shell::needs_setup_hint() {
        issues.push("shell integration not detected; run 'goto setup'".to_string());
    } else {
        println!("[ok] shell integration detected");
    }

    if issues.is_empty() {
        println!("doctor: all checks passed");
        return Ok(());
    }

    report_doctor(issues);
    bail!("doctor found issues")
}

fn report_doctor(issues: Vec<String>) {
    for issue in issues {
        eprintln!("[issue] {issue}");
    }
}

fn find_namespace_index(cfg: &config::ConfigFile, lookup: &str) -> Option<usize> {
    cfg.namespace
        .iter()
        .position(|entry| entry.name.eq_ignore_ascii_case(lookup))
}

fn find_namespace_or_alias_index(cfg: &config::ConfigFile, lookup: &str) -> Option<usize> {
    if let Some(index) = find_namespace_index(cfg, lookup) {
        return Some(index);
    }

    cfg.namespace.iter().position(|entry| {
        entry.aliases.as_ref().is_some_and(|aliases| {
            aliases
                .iter()
                .any(|alias| alias.eq_ignore_ascii_case(lookup))
        })
    })
}
