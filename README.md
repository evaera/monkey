# monkey
(mon-key)

Switches an external monitor's active input over DDC/CI (VCP `0x60`). A single
small binary for Windows and macOS, meant to replace flaky vendor OSD utilities.
The motivating case is two machines sharing one monitor, one on DisplayPort and
one on USB-C.

```
cargo build --release    # -> target/release/monkey(.exe)
```

## Usage

```
monkey read           current input value
monkey list           every display and its current input
monkey set <input>    switch by name (from [inputs]) or a raw number
monkey <input>        shorthand for set, e.g. `monkey usbc`, `monkey 16`
monkey toggle         flip between the two `toggle` inputs
monkey listen         daemon: global hotkeys, plus the [usb] watcher if set
monkey watch          just the [usb] watcher
monkey usb            list connected USB devices (to find a VID:PID)
monkey startup        run the daemon at login (--remove to undo)
```

`-m/--model <substr>` picks the display by EDID model name; `-c/--config <path>`
points at a specific file. Exit codes: 0 ok, 1 DDC/config failure, 2 bad args.

## Config

`monkey.toml` is found by walking up from the working directory, so a copy in the
repo or in the home directory is picked up; `--config` and `$MONKEY_CONFIG`
override that. Without a file, the built-in defaults below apply.

```toml
model = "MPG"            # EDID model substring used to pick the display
toggle = ["dp", "usbc"]  # what `toggle` flips between

[inputs]                 # name -> VCP 0x60 value
dp = 15
usbc = 16
hdmi1 = 17
hdmi2 = 18

[hotkeys]                # combo -> input, for `monkey listen`
"ctrl+alt+1" = "dp"
"ctrl+alt+2" = "usbc"

[usb]                    # follow a USB device across machines
device = "3297:1977"     # VID:PID, from `monkey usb`
on_connect = "dp"        # switch here when it appears
on_disconnect = "usbc"   # hand off when it leaves
wake_on_connect = true   # wake this machine's screen first (see below)
# wake_on_disconnect = false
# wake_settle_ms = 500   # panel light-up time before the switch
```

Names are arbitrary keys in `[inputs]`, so this works on any monitor once its
values are known. On the MSI MPG panel this config targets, 15 = DisplayPort and
16 = USB-C are confirmed; the HDMI values are unverified. To find a value, switch
to that input and run `monkey list`.

The same panel can appear twice in `list`, once per backend (a winapi "Generic
PnP Monitor" and an nvapi/macos entry with the real name). `model` selects which
one gets driven, so if a switch does not take, try the other backend's name.

## Two machines

Most monitors only accept DDC on the input that is currently on screen, so a
machine that is not showing cannot switch the monitor to itself. With a KM switch
that does not move video, bind each machine to switch *away*: the DisplayPort
machine runs `usbc`, the USB-C machine runs `dp`. Whichever machine is on screen
is the one that can hand off.

## Following the keyboard

With `[usb]` configured, the daemon polls for a USB device (the keyboard on the
KM switch) and switches the monitor when it connects or disconnects, so the
video follows the keyboard automatically. `monkey listen` runs this alongside
the hotkeys in one daemon; `monkey watch` runs it alone. `monkey usb` prints
the VID:PID pairs to put in `device`.

`wake_on_connect` / `wake_on_disconnect` wake this machine's screen on the
event that hands it the monitor. Without this, a machine whose displays have
gone to sleep offers no signal, and the monitor bounces straight back to the
other input. Windows wakes via a synthetic one-pixel mouse nudge; macOS runs
`caffeinate -u`. `wake_settle_ms` (default 500) is how long the panel gets to
light up before the input switch is sent.

## Hotkeys

`monkey listen` registers `[hotkeys]` system-wide and switches on each press.
`monkey startup` makes the daemon run at login: `listen` when hotkeys are
configured, `watch` when only `[usb]` is (an `HKCU\...\Run` entry on Windows, a
LaunchAgent on macOS, both pinned to the config in use); `monkey startup --remove`
undoes it. On Windows the logon instance hides its console window. Alternatively, bind the commands in an existing hotkey tool,
such as AutoHotkey `^!2::Run('monkey.exe usbc')` or skhd `cmd + alt - 1 : monkey dp`.

## License

[MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE).
