use anyhow::Result;
use std::path::Path;

pub fn install(exe: &Path, config: Option<&Path>) -> Result<String> {
    platform::install(exe, config)
}

pub fn uninstall() -> Result<String> {
    platform::uninstall()
}

#[cfg(windows)]
mod platform {
    use anyhow::{Context, Result};
    use std::path::Path;
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_WRITE};

    const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    // Task Manager records each entry's enabled state here; it outlives the Run
    // value, so a row lingers in the Startup tab unless it is cleared too.
    const APPROVED_KEY: &str =
        r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";
    const VALUE: &str = "monkey";

    pub fn install(exe: &Path, config: Option<&Path>) -> Result<String> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (run, _) = hkcu
            .create_subkey(RUN_KEY)
            .context("opening HKCU Run key")?;
        run.set_value(VALUE, &command(exe, config))
            .context("writing startup value")?;
        Ok(format!(r"HKCU\{RUN_KEY}\{VALUE}"))
    }

    pub fn uninstall() -> Result<String> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(run) = hkcu.open_subkey_with_flags(RUN_KEY, KEY_WRITE) {
            match run.delete_value(VALUE) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(anyhow::Error::new(e).context("deleting startup value")),
            }
        }
        if let Ok(approved) = hkcu.open_subkey_with_flags(APPROVED_KEY, KEY_WRITE) {
            let _ = approved.delete_value(VALUE);
        }
        Ok(format!(r"HKCU\{RUN_KEY}\{VALUE}"))
    }

    // quote each path in case it has spaces
    fn command(exe: &Path, config: Option<&Path>) -> String {
        let mut s = format!("\"{}\"", exe.display());
        if let Some(c) = config {
            s.push_str(&format!(" --config \"{}\"", c.display()));
        }
        s.push_str(" listen");
        s
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn command_quotes_paths() {
            let c = command(
                Path::new(r"C:\a b\monkey.exe"),
                Some(Path::new(r"C:\u\monkey.toml")),
            );
            assert_eq!(
                c,
                r#""C:\a b\monkey.exe" --config "C:\u\monkey.toml" listen"#
            );
        }
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use anyhow::{Context, Result};
    use std::path::{Path, PathBuf};

    fn plist_path() -> Result<PathBuf> {
        let home = std::env::home_dir().context("could not find the home directory")?;
        Ok(home
            .join("Library")
            .join("LaunchAgents")
            .join("monkey.plist"))
    }

    pub fn install(exe: &Path, config: Option<&Path>) -> Result<String> {
        let path = plist_path()?;
        std::fs::write(&path, plist(exe, config))
            .with_context(|| format!("writing {}", path.display()))?;
        Ok(path.display().to_string())
    }

    pub fn uninstall() -> Result<String> {
        let path = plist_path()?;
        if path.exists() {
            std::fs::remove_file(&path).with_context(|| format!("removing {}", path.display()))?;
        }
        Ok(path.display().to_string())
    }

    fn plist(exe: &Path, config: Option<&Path>) -> String {
        let mut args = vec![format!("        <string>{}</string>", exe.display())];
        if let Some(c) = config {
            args.push("        <string>--config</string>".to_string());
            args.push(format!("        <string>{}</string>", c.display()));
        }
        args.push("        <string>listen</string>".to_string());
        let args = args.join("\n");
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key><string>monkey</string>
    <key>ProgramArguments</key>
    <array>
{args}
    </array>
    <key>RunAtLoad</key><true/>
    <key>KeepAlive</key><true/>
</dict>
</plist>
"#
        )
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
mod platform {
    use anyhow::{Result, bail};
    use std::path::Path;

    pub fn install(_exe: &Path, _config: Option<&Path>) -> Result<String> {
        bail!("`monkey startup` only works on Windows and macOS");
    }

    pub fn uninstall() -> Result<String> {
        bail!("`monkey startup` only works on Windows and macOS");
    }
}
