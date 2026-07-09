# monkey
(mon-key)

Switches an external monitor's active input over DDC/CI (VCP `0x60`). One small
binary for Windows and macOS, instead of flaky vendor OSD utilities. The
motivating case is two machines sharing one monitor, one on DisplayPort and one
on USB-C.

## build

```
cargo build --release
```

## use

```
monkey <input>        switch by name or raw value, e.g. `monkey usbc`, `monkey 16`
monkey toggle         flip between the two `toggle` inputs
monkey read           current input value
monkey list           every display and its current input
monkey listen         daemon: global hotkeys, plus the [usb] watcher if set
monkey watch          just the [usb] watcher
monkey usb            list connected USB devices (to find a VID:PID)
monkey startup        run the daemon at login (--remove to undo)
```

`-m/--model <substr>` picks the display by EDID model name; `-c/--config <path>`
points at a specific file. Exit codes: 0 ok, 1 DDC/config failure, 2 bad args.

`startup` registers `listen` at login (`watch` when only `[usb]` is configured):
an `HKCU\...\Run` entry on Windows, a LaunchAgent on macOS, pinned to the config
in use, windowless on Windows.

## config

`monkey.toml` is found by walking up from the working directory; `--config` and
`$MONKEY_CONFIG` override.

```toml
model = "MPG"            # EDID model substring used to pick the display
toggle = ["dp", "usbc"]  # what `toggle` flips between

[inputs]                 # name -> VCP 0x60 value; find them with `monkey list`
dp = 15
usbc = 16

[hotkeys]                # combo -> input, for `monkey listen`
"ctrl+alt+1" = "dp"
"ctrl+alt+2" = "usbc"

[usb]                    # follow a USB device across machines
device = "3297:1977"     # VID:PID, from `monkey usb`
on_connect = "dp"        # switch here when it appears
on_disconnect = "usbc"   # hand off when it leaves
wake_on_connect = true   # wake this machine's screen first
# wake_on_disconnect = false
# wake_settle_ms = 500   # panel light-up time before the switch
```

The same panel can appear once per backend in `list` (a winapi "Generic PnP
Monitor" and an nvapi/macos entry with the real name); `model` picks which one
gets driven, so if a switch does not take, try the other name.

## two machines

Most monitors only accept DDC on the input currently on screen, so a machine
that is not showing cannot pull the monitor to itself. Bind each machine to
switch *away*: the DisplayPort machine runs `usbc`, the USB-C machine runs
`dp`. Point `[usb]` at the keyboard on a KM switch and the video follows the
keyboard. `wake_on_connect` wakes the gaining machine's screen first; asleep,
it offers no signal and the monitor bounces straight back.

## license

[MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE).
