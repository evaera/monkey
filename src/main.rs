mod cli;
mod config;
mod hotkey;
mod monitor;
mod startup;

use anyhow::{Context, Result, bail};
use clap::Parser;
use cli::{Cli, Command};
use config::Config;
use monitor::InputSource;
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    // clap handles --help/--version and usage errors itself
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("monkey: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<()> {
    let Cli {
        model,
        config,
        command,
    } = cli;

    let cfg = config::load(config.as_deref())?;
    let model = model.as_deref().or(cfg.model.as_deref());

    match command {
        Command::Read => cmd_read(&cfg, model),
        Command::List => cmd_list(&cfg),
        Command::Set { input } => cmd_set(&cfg, model, &input),
        Command::Toggle => cmd_toggle(&cfg, model),
        Command::Listen => hotkey::listen(&cfg, model),
        Command::Startup { remove } => cmd_startup(&cfg, config.as_deref(), remove),
        Command::Switch(args) => match args.as_slice() {
            [input] => cmd_set(&cfg, model, input),
            _ => bail!("expected one input, e.g. `monkey usbc`"),
        },
    }
}

fn cmd_startup(cfg: &Config, config_path: Option<&Path>, remove: bool) -> Result<()> {
    if remove {
        println!("removed startup entry ({})", startup::uninstall()?);
        return Ok(());
    }
    if cfg.hotkeys.is_empty() {
        eprintln!(
            "monkey: warning: config has no [hotkeys], so listen exits at once; add some and re-run"
        );
    }
    let exe = std::env::current_exe().context("finding the monkey executable")?;
    let config_abs = config::resolve_path(config_path)
        .map(std::path::absolute)
        .transpose()
        .context("resolving the config path")?;
    let at = startup::install(&exe, config_abs.as_deref())?;
    println!("registered `monkey listen` at login ({at})");
    println!("takes effect at your next login");
    #[cfg(target_os = "macos")]
    println!("start it now with: launchctl load \"{at}\"");
    Ok(())
}

fn cmd_read(cfg: &Config, model: Option<&str>) -> Result<()> {
    let mut mon = monitor::open(model)?;
    let value = mon.get_input()?;
    match cfg.name_for_value(value) {
        Some(name) => println!("current input: {value} ({name}) on {}", mon.label),
        None => println!("current input: {value} on {}", mon.label),
    }
    Ok(())
}

fn cmd_list(cfg: &Config) -> Result<()> {
    let reports = monitor::report_all()?;
    for (i, r) in reports.iter().enumerate() {
        let input = match &r.input {
            Ok(v) => match cfg.name_for_value(*v) {
                Some(name) => format!("{v} ({name})"),
                None => v.to_string(),
            },
            Err(e) => format!("unavailable ({e})"),
        };
        println!("[{i}] {}", r.label);
        println!("    backend: {}   id: {}", r.backend, r.id);
        if let Some(serial) = &r.serial {
            println!("    serial:  {serial}");
        }
        println!("    input (0x60): {input}");
    }
    Ok(())
}

fn cmd_set(cfg: &Config, model: Option<&str>, target: &str) -> Result<()> {
    let value = cfg.resolve_input(target)?;
    let mut mon = monitor::open(model)?;
    mon.set_input(value)?;
    println!("switched {} to {target} ({value})", mon.label);
    Ok(())
}

fn cmd_toggle(cfg: &Config, model: Option<&str>) -> Result<()> {
    let mut mon = monitor::open(model)?;
    let current = mon.get_input()?;
    let target = cfg.toggle_target(current)?;
    mon.set_input(target)?;
    let name = cfg.name_for_value(target).unwrap_or("?");
    println!("toggled {} from {current} to {target} ({name})", mon.label);
    Ok(())
}
