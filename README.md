```
                 ██╗ ██╗      █████╗ ██╗     ██╗
   __ _ _   _  ╚██╗██╔╝     ██╔══██╗██║     ██║
  / _` | | | |  ╚███╔╝█████╗███████║██║     ██║
 | (_| | |_| |  ██╔██╗╚════╝██╔══██║██║     ██║
  \__, |\__,_| ██╔╝ ██╗     ██║  ██║███████╗███████╗
     |_|       ╚═╝  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝
```

A fast, no-nonsense macOS command-line tool that quits all running GUI applications in one shot. Written in Rust.

> **macOS only** — uses native AppKit/NSWorkspace APIs via Objective-C bindings. Will not compile on Linux or Windows.

## Install

```bash
cargo build --release
cp target/release/quit-all /usr/local/bin/
```

## Usage

```bash
quit-all              # Gracefully quit all apps
quit-all --force      # Force-terminate all apps (no save prompts)
quit-all --dry-run    # Preview what would be quit
```

## Config

Apps can be whitelisted via `~/.config/quit-all.json`:

```json
{
  "whitelist": [
    "com.apple.Safari",
    "com.apple.Terminal"
  ]
}
```

Whitelisted apps are skipped during quit, just like the built-in safelist (Finder, System Preferences).

To find an app's bundle ID:

```bash
osascript -e 'id of app "Safari"'
```

## Built-in Safelist

These apps are **never** quit, regardless of config:

- `com.apple.finder`
- `com.apple.systempreferences`
