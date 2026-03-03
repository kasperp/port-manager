# Port Manager

A Windows tray app for keeping SSH port forwards alive.

## What it does

- Forwards local ports over SSH (same as running `ssh -N -L ...` by hand)
- Watches for dead tunnels and reconnects them automatically
- Lives in the system tray — right-click to see which ports are up at a glance
- Starts with Windows if you want it to

## Getting started

Download the installer from the [latest release](../../releases/latest) and run it.

On first launch, fill in your server details (host, SSH port, username) and add the ports you want forwarded. Hit **Start All** and check the tray icon — green means all ports are active.

Tick **Run on startup** once everything's working the way you want.

## Profiles

Port Manager supports multiple connection profiles, so you can manage port forwards to different servers without re-entering settings each time.

### How profiles work

Each profile stores its own:
- **Host** and **SSH port**
- **Username**
- **List of forwarded ports**

The active profile determines which SSH tunnels are managed. Only one profile is active at a time.

### Switching profiles

Use the **Profile** dropdown at the top of the app to switch between profiles. When you switch:
1. All tunnels for the current profile are stopped
2. The new profile becomes active
3. You can then start tunnels for the new profile with **Start All**

### Creating a profile

Click the **+** button next to the profile dropdown, type a name, and press **Create**. The new profile starts with empty connection settings — fill in the host, user, and SSH port, then add your forwarded ports.

### Importing from SSH config

If you have hosts defined in `~/.ssh/config`, Port Manager can import them as profiles:

1. Click the **SSH** button next to the profile dropdown
2. A list of hosts from your SSH config appears (wildcard entries like `Host *` are excluded)
3. Click a host to create a profile pre-filled with the hostname, user, and port from your SSH config

The imported profile will use the SSH host alias as its name. If a profile with that name already exists, a numbered suffix is added (e.g. "myserver (2)").

Only hosts that haven't already been imported are shown in the list. The SSH button is hidden if there are no importable hosts.

### Deleting a profile

Select the profile you want to remove in the dropdown, then click the **x** button. You'll be asked to confirm. If you delete the active profile, Port Manager stops its tunnels and switches to the first remaining profile.

You cannot delete the last profile — there must always be at least one.

### Profile in the tray

The system tray context menu shows which profile is currently active, along with the port statuses for that profile.

## Configuration

Settings are saved to `%APPDATA%\com.portmanager.app\config.json`. The config file stores all profiles and tracks which one is active:

```json
{
  "active_profile": "production",
  "profiles": [
    {
      "name": "production",
      "host": "prod.example.com",
      "user": "deploy",
      "ssh_port": 22,
      "ports": [5432, 6379]
    },
    {
      "name": "staging",
      "host": "staging.example.com",
      "user": "deploy",
      "ssh_port": 22,
      "ports": [5432]
    }
  ]
}
```

SSH keys are handled by your existing SSH agent / `~/.ssh/config` — Port Manager just calls `ssh` the same way you would from a terminal.

### Migration from older versions

If you're upgrading from an older version that used a flat config (single host/user/ports), your settings are automatically migrated into a profile named "Default" on first launch. No action needed.

## Building from source

You'll need [Rust](https://rustup.rs/) and Node.js 18+.

```sh
npm install
npm run tauri dev    # dev build with hot reload
npm run tauri build  # release build -> src-tauri/target/release/
```

The release workflow builds and publishes automatically when the version in `tauri.conf.json` is bumped and pushed to main.
