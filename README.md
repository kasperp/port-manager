# Port Manager

A small Windows tray app for keeping SSH port forwards alive.

I got tired of my PowerShell script taking five seconds to open and rebuilding its UI from scratch every time the reconnect timer fired. This is the rewrite — Rust backend, Svelte frontend, ships as a single exe under 10 MB.

![tray menu showing port statuses](.github/screenshot.png)

## What it does

- Forwards local ports over SSH (same as running `ssh -N -L ...` by hand)
- Watches for dead tunnels and reconnects them automatically
- Lives in the system tray — right-click to see which ports are up at a glance
- Starts with Windows if you want it to

## Getting started

Download the installer from the [latest release](../../releases/latest) and run it. That's it.

On first launch, fill in your server details (host, SSH port, username) and add the ports you want forwarded. Hit **Start All** and check the tray icon — green means all ports are active.

Tick **Run on startup** once everything's working the way you want.

## Configuration

Settings are saved to `%APPDATA%\com.portmanager.app\config.json`. You don't normally need to touch this file directly, but it's plain JSON if you want to.

SSH keys are handled by your existing SSH agent / `~/.ssh/config` — Port Manager just calls `ssh` the same way you would from a terminal.

## Building from source

You'll need [Rust](https://rustup.rs/) and Node.js 18+.

```sh
npm install
npm run tauri dev    # dev build with hot reload
npm run tauri build  # release build → src-tauri/target/release/
```

The release workflow builds and publishes automatically when the version in `tauri.conf.json` is bumped and pushed to main.

## Why not just use a script?

The old PowerShell version worked fine, but it was slow to start (~5 seconds), couldn't update the UI without rebuilding the whole window, and left SSH processes orphaned on exit. A compiled app with a proper process lifecycle made more sense once the PS1 got long enough to maintain.
