use clap::{CommandFactory, Parser, Subcommand};

use crate::shell;

#[derive(Parser)]
#[command(name = "goto")]
#[command(version)]
#[command(about = "Navigate to projects using namespace-based paths.")]
#[command(
    override_usage = "goto <namespace>/<path>\n    goto list [namespace]\n    goto setup\n    goto uninstall\n    goto config-path\n    goto doctor\n    goto add <name> <path> [--alias <value> ...]\n    goto remove <name>\n    goto rename <old> <new>\n    goto set-path <name> <path>\n    goto alias-add <namespace> <alias>\n    goto alias-remove <namespace> <alias>\n    goto list-raw"
)]
#[command(allow_external_subcommands = true)]
pub(crate) struct Goto {
    #[command(subcommand)]
    pub(crate) command: Option<Command>,
}

#[derive(Subcommand)]
pub(crate) enum Command {
    /// List available namespaces and projects
    List {
        /// Optional namespace to scope the list
        #[arg(value_name = "NAMESPACE")]
        namespace: Option<String>,
    },

    /// Set up shell integration (zsh)
    Setup,

    /// Remove shell integration (zsh)
    Uninstall,

    /// Print the active configuration file path
    ConfigPath,

    /// Run diagnostics for config and shell integration
    Doctor,

    /// Add a namespace
    Add {
        /// Namespace name
        #[arg(value_name = "NAME")]
        name: String,

        /// Namespace root path
        #[arg(value_name = "PATH")]
        path: String,

        /// Optional aliases (repeatable)
        #[arg(long = "alias", short = 'a', value_name = "ALIAS")]
        aliases: Vec<String>,
    },

    /// Remove a namespace
    Remove {
        /// Namespace name or alias
        #[arg(value_name = "NAME")]
        name: String,
    },

    /// Rename a namespace
    Rename {
        /// Existing namespace name
        #[arg(value_name = "OLD")]
        old: String,

        /// New namespace name
        #[arg(value_name = "NEW")]
        new: String,
    },

    /// Update namespace root path
    SetPath {
        /// Namespace name
        #[arg(value_name = "NAME")]
        name: String,

        /// New namespace root path
        #[arg(value_name = "PATH")]
        path: String,
    },

    /// Add an alias to a namespace
    AliasAdd {
        /// Namespace name
        #[arg(value_name = "NAME")]
        namespace: String,

        /// Alias to add
        #[arg(value_name = "ALIAS")]
        alias: String,
    },

    /// Remove an alias from a namespace
    AliasRemove {
        /// Namespace name
        #[arg(value_name = "NAME")]
        namespace: String,

        /// Alias to remove
        #[arg(value_name = "ALIAS")]
        alias: String,
    },

    /// Print effective config as TOML
    ListRaw,

    /// Generate dynamic completions for the current target (internal)
    #[command(name = "__complete", hide = true)]
    CompleteTargets {
        /// Current word to complete
        #[arg(value_name = "PARTIAL")]
        partial: Option<String>,
    },

    #[command(external_subcommand)]
    External(Vec<String>),
}

pub(crate) fn print_help_with_setup_hint() {
    if let Err(error) = Goto::command().print_help() {
        eprintln!("failed to print help: {error}");
    }
    println!();

    if shell::needs_setup_hint() {
        println!("\x1b[1;33m⚠ Shell integration not detected\x1b[0m");
        println!();
        println!("Run \x1b[1mgoto setup\x1b[0m to enable:");
        println!("  • Automatic directory navigation (cd to resolved paths)");
        println!("  • Tab completion for namespaces and projects");
        println!();
        println!("Without setup, goto only prints paths to stdout.");
    }
}
