use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const FILENAME: &str = "monkey.toml";

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default = "default_inputs")]
    pub inputs: BTreeMap<String, u16>,
    #[serde(default = "default_toggle")]
    pub toggle: Vec<String>,
    #[serde(default)]
    pub hotkeys: BTreeMap<String, String>,
    #[serde(default)]
    pub usb: Option<UsbWatch>,
}

/// `monkey watch`: switch input when a USB device connects/disconnects.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UsbWatch {
    /// USB vendor:product in hex, e.g. "3297:1977".
    pub device: String,
    #[serde(default)]
    pub on_connect: Option<String>,
    #[serde(default)]
    pub on_disconnect: Option<String>,
}

impl UsbWatch {
    pub fn ids(&self) -> Result<(u16, u16)> {
        let (v, p) = self.device.split_once(':').with_context(|| {
            format!(
                "`device` must be VID:PID, e.g. 3297:1977 (got '{}')",
                self.device
            )
        })?;
        let vid =
            u16::from_str_radix(v.trim(), 16).with_context(|| format!("bad vendor id '{v}'"))?;
        let pid =
            u16::from_str_radix(p.trim(), 16).with_context(|| format!("bad product id '{p}'"))?;
        Ok((vid, pid))
    }
}

fn default_inputs() -> BTreeMap<String, u16> {
    BTreeMap::from([
        ("dp".to_string(), 15),
        ("usbc".to_string(), 16),
        ("hdmi1".to_string(), 17),
        ("hdmi2".to_string(), 18),
    ])
}

fn default_toggle() -> Vec<String> {
    vec!["dp".to_string(), "usbc".to_string()]
}

impl Default for Config {
    fn default() -> Self {
        Config {
            model: Some("MPG".to_string()),
            inputs: default_inputs(),
            toggle: default_toggle(),
            hotkeys: BTreeMap::new(),
            usb: None,
        }
    }
}

impl Config {
    // name (case-insensitive) or a raw number
    pub fn resolve_input(&self, target: &str) -> Result<u16> {
        let t = target.trim();
        if let Ok(v) = t.parse::<u16>() {
            return Ok(v);
        }
        for (name, v) in &self.inputs {
            if name.eq_ignore_ascii_case(t) {
                return Ok(*v);
            }
        }
        let known = self.inputs.keys().cloned().collect::<Vec<_>>().join(", ");
        bail!("unknown input '{target}'. known: {known} (or pass a number)");
    }

    pub fn name_for_value(&self, value: u16) -> Option<&str> {
        self.inputs
            .iter()
            .find(|(_, v)| **v == value)
            .map(|(name, _)| name.as_str())
    }

    // flip; when on neither, pick the first
    pub fn toggle_target(&self, current: u16) -> Result<u16> {
        let a = self.resolve_input(&self.toggle[0])?;
        let b = self.resolve_input(&self.toggle[1])?;
        Ok(if current == a { b } else { a })
    }

    fn validate(&self) -> Result<()> {
        if self.toggle.len() != 2 {
            bail!(
                "`toggle` needs exactly two inputs, got {}",
                self.toggle.len()
            );
        }
        for name in &self.toggle {
            self.resolve_input(name)
                .with_context(|| format!("`toggle` references '{name}'"))?;
        }
        if let Some(usb) = &self.usb {
            usb.ids().context("in [usb]")?;
            for name in [&usb.on_connect, &usb.on_disconnect].into_iter().flatten() {
                self.resolve_input(name)
                    .with_context(|| format!("[usb] references '{name}'"))?;
            }
        }
        Ok(())
    }
}

pub fn load(explicit: Option<&Path>) -> Result<Config> {
    let Some(path) = resolve_path(explicit) else {
        return Ok(Config::default());
    };
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let config: Config =
        toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
    config
        .validate()
        .with_context(|| format!("in {}", path.display()))?;
    Ok(config)
}

pub fn resolve_path(explicit: Option<&Path>) -> Option<PathBuf> {
    if let Some(p) = explicit {
        return Some(p.to_path_buf());
    }
    if let Ok(env) = std::env::var("MONKEY_CONFIG")
        && !env.is_empty()
    {
        return Some(PathBuf::from(env));
    }
    find_upwards(&std::env::current_dir().ok()?)
}

// start, then each parent up to the root
fn find_upwards(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .map(|dir| dir.join(FILENAME))
        .find(|p| p.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg_from(src: &str) -> Config {
        let c: Config = toml::from_str(src).expect("parse");
        c.validate().expect("validate");
        c
    }

    #[test]
    fn resolve() {
        let c = Config::default();
        assert_eq!(c.resolve_input("usbc").unwrap(), 16);
        assert_eq!(c.resolve_input("USBC").unwrap(), 16);
        assert_eq!(c.resolve_input(" 16 ").unwrap(), 16);
        assert!(c.resolve_input("hdmi9").is_err());
    }

    #[test]
    fn toggle() {
        let c = Config::default();
        assert_eq!(c.toggle_target(15).unwrap(), 16);
        assert_eq!(c.toggle_target(16).unwrap(), 15);
        assert_eq!(c.toggle_target(99).unwrap(), 15);
    }

    #[test]
    fn custom_config() {
        let c = cfg_from("model = \"Dell\"\ntoggle = [\"l\", \"r\"]\n\n[inputs]\nl = 17\nr = 18\n");
        assert_eq!(c.model.as_deref(), Some("Dell"));
        assert_eq!(c.resolve_input("l").unwrap(), 17);
        assert_eq!(c.toggle_target(17).unwrap(), 18);
    }

    #[test]
    fn rejects_bad_config() {
        assert!(toml::from_str::<Config>("notathing = 1\n").is_err());
        let one = toml::from_str::<Config>("toggle = [\"dp\"]\n").unwrap();
        assert!(one.validate().is_err());
    }

    #[test]
    fn find_upwards() {
        let base = std::env::temp_dir().join(format!("monkey-test-{}", std::process::id()));
        let deep = base.join("a/b/c");
        std::fs::create_dir_all(&deep).unwrap();
        let cfg = base.join("a").join(FILENAME);
        std::fs::write(&cfg, "model = \"X\"\n").unwrap();

        assert_eq!(super::find_upwards(&deep), Some(cfg));
        let _ = std::fs::remove_dir_all(&base);
    }
}
