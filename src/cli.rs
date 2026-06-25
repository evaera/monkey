use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug, PartialEq, Eq)]
#[command(
    name = "monkey",
    version,
    about = "Switch a monitor's input over DDC/CI",
    arg_required_else_help = true
)]
pub struct Cli {
    /// Match the display by EDID model substring
    #[arg(short, long, global = true, value_name = "SUBSTR")]
    pub model: Option<String>,

    /// Path to a monkey.toml
    #[arg(short, long, global = true, value_name = "PATH")]
    pub config: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug, PartialEq, Eq)]
pub enum Command {
    /// Show the current input
    Read,
    /// List displays and their inputs
    #[command(alias = "ls")]
    List,
    /// Switch input by name or number (or just `monkey <input>`)
    Set {
        #[arg(value_name = "INPUT")]
        input: String,
    },
    /// Toggle between the two `toggle` inputs
    Toggle,
    /// Watch the config's global hotkeys
    Listen,
    /// Run `monkey listen` at login (--remove to undo)
    Startup {
        /// Remove the startup entry instead of adding it
        #[arg(long)]
        remove: bool,
    },
    // bare `monkey <input>`, shorthand for `set <input>`
    #[command(external_subcommand)]
    Switch(Vec<String>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn clap_defs_are_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn parses_global_flag_and_subcommand() {
        let cli = Cli::try_parse_from(["monkey", "-m", "Dell", "set", "16"]).unwrap();
        assert_eq!(cli.model.as_deref(), Some("Dell"));
        assert_eq!(cli.command, Command::Set { input: "16".into() });
    }

    #[test]
    fn bare_input_is_shorthand_for_set() {
        let cli = Cli::try_parse_from(["monkey", "usbc"]).unwrap();
        assert_eq!(cli.command, Command::Switch(vec!["usbc".into()]));
    }
}
