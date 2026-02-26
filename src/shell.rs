use anyhow::{Context, Result};
use std::{env, fs, io::Write, path::PathBuf};

const ZSH_INTEGRATION: &str = include_str!("../shell/goto.zsh");
const ZSH_MARKER_START: &str = "# >>> goto integration (managed by goto) >>>";
const ZSH_MARKER_END: &str = "# <<< goto integration (managed by goto) <<<";

pub(crate) fn needs_setup_hint() -> bool {
    zshrc_path()
        .ok()
        .and_then(|path| fs::read_to_string(path).ok())
        .map(|content| !content.contains(ZSH_MARKER_START))
        .unwrap_or(true)
}

pub(crate) fn setup_zsh() -> Result<()> {
    let zshrc = zshrc_path()?;
    let existing = fs::read_to_string(&zshrc).unwrap_or_default();

    if existing.contains(ZSH_MARKER_START) {
        println!("goto already configured in {}", zshrc.display());
        return Ok(());
    }

    if let Some(parent) = zshrc.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&zshrc)?;

    if !existing.is_empty() && !existing.ends_with('\n') {
        writeln!(file)?;
    }

    let last_line_blank = existing
        .lines()
        .last()
        .map(|line| line.trim().is_empty())
        .unwrap_or(true);
    if !existing.is_empty() && !last_line_blank {
        writeln!(file)?;
    }

    writeln!(file, "{ZSH_INTEGRATION}")?;
    println!(
        "Added goto helper to {}. Restart your shell.",
        zshrc.display()
    );
    Ok(())
}

pub(crate) fn uninstall_zsh() -> Result<()> {
    let zshrc = zshrc_path()?;
    let existing = fs::read_to_string(&zshrc).unwrap_or_default();

    if !existing.contains(ZSH_MARKER_START) {
        println!("goto helper not found in {}", zshrc.display());
        return Ok(());
    }

    let mut output = Vec::new();
    let mut in_block = false;

    for line in existing.lines() {
        if line.trim_end() == ZSH_MARKER_START {
            in_block = true;
            continue;
        }

        if in_block {
            if line.trim_end() == ZSH_MARKER_END {
                in_block = false;
            }
            continue;
        }

        output.push(line);
    }

    let mut rendered = output.join("\n");
    if !rendered.is_empty() {
        rendered.push('\n');
    }

    fs::write(&zshrc, rendered)?;
    println!("Removed goto helper from {}", zshrc.display());
    Ok(())
}

fn zshrc_path() -> Result<PathBuf> {
    let home = env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .context("HOME is not set")?;
    let zdotdir = env::var_os("ZDOTDIR").map(PathBuf::from).unwrap_or(home);
    Ok(zdotdir.join(".zshrc"))
}
