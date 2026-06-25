use crate::config::Config;
use crate::monitor::{self, InputSource};
use anyhow::{Context, Result, anyhow, bail};
use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use std::collections::HashMap;
use std::str::FromStr;

struct Binding {
    combo: String,
    target: String,
    value: u16,
}

pub fn listen(config: &Config, model: Option<&str>) -> Result<()> {
    if config.hotkeys.is_empty() {
        bail!("no [hotkeys] in monkey.toml, e.g. \"ctrl+alt+1\" = \"dp\"");
    }
    #[cfg(windows)]
    hide_console_if_orphan();

    let manager = GlobalHotKeyManager::new().context("starting the hotkey manager")?;
    let mut bindings: HashMap<u32, Binding> = HashMap::new();
    for (combo, target) in &config.hotkeys {
        let hotkey = HotKey::from_str(combo).map_err(|e| anyhow!("bad hotkey '{combo}': {e}"))?;
        let value = config
            .resolve_input(target)
            .with_context(|| format!("hotkey '{combo}'"))?;
        manager
            .register(hotkey)
            .with_context(|| format!("registering '{combo}' (already taken?)"))?;
        println!("{combo:<16} -> {target} ({value})");
        bindings.insert(
            hotkey.id(),
            Binding {
                combo: combo.clone(),
                target: target.clone(),
                value,
            },
        );
    }
    println!("listening for {} hotkey(s), ctrl+c to quit", bindings.len());

    run(|id| {
        if let Some(b) = bindings.get(&id) {
            // re-pick the display; it may have dropped after sleep
            let result = monitor::open(model).and_then(|mut m| {
                m.set_input(b.value)?;
                Ok(m.label)
            });
            match result {
                Ok(label) => println!("{} -> {} ({}) on {label}", b.combo, b.target, b.value),
                Err(e) => eprintln!("{} -> {} failed: {e:#}", b.combo, b.target),
            }
        }
    })
}

// Launched at logon from the registry, listen owns a fresh console window that
// would otherwise stay open all session. If monkey is the only process on the
// console (no parent terminal), hide it; from a real shell, leave output visible.
#[cfg(windows)]
fn hide_console_if_orphan() {
    use windows_sys::Win32::System::Console::{GetConsoleProcessList, GetConsoleWindow};
    use windows_sys::Win32::UI::WindowsAndMessaging::{SW_HIDE, ShowWindow};

    unsafe {
        let mut pids = [0u32; 2];
        if GetConsoleProcessList(pids.as_mut_ptr(), pids.len() as u32) == 1 {
            let hwnd = GetConsoleWindow();
            if !hwnd.is_null() {
                ShowWindow(hwnd, SW_HIDE);
            }
        }
    }
}

#[cfg(windows)]
fn run<F: FnMut(u32)>(mut on_press: F) -> Result<()> {
    // pump this thread's messages so global-hotkey's hidden window proc runs
    use windows_sys::Win32::UI::WindowsAndMessaging::{DispatchMessageW, GetMessageW, MSG};

    let events = GlobalHotKeyEvent::receiver();
    let mut msg: MSG = unsafe { core::mem::zeroed() };
    loop {
        match unsafe { GetMessageW(&mut msg, core::ptr::null_mut(), 0, 0) } {
            0 => return Ok(()),
            -1 => bail!("GetMessageW failed"),
            _ => unsafe {
                DispatchMessageW(&msg);
            },
        }
        while let Ok(e) = events.try_recv() {
            if e.state == HotKeyState::Pressed {
                on_press(e.id);
            }
        }
    }
}

#[cfg(target_os = "macos")]
fn run<F: FnMut(u32)>(mut on_press: F) -> Result<()> {
    // run the main loop in slices; the Carbon handler fills the channel
    use core_foundation_sys::runloop::{CFRunLoopRunInMode, kCFRunLoopDefaultMode};

    let events = GlobalHotKeyEvent::receiver();
    loop {
        unsafe { CFRunLoopRunInMode(kCFRunLoopDefaultMode, 0.2, 0) };
        while let Ok(e) = events.try_recv() {
            if e.state == HotKeyState::Pressed {
                on_press(e.id);
            }
        }
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
fn run<F: FnMut(u32)>(_on_press: F) -> Result<()> {
    bail!("`monkey listen` only works on Windows and macOS");
}
