# Port Manager

A Windows GUI application for managing SSH port forwards with auto-reconnect capability.

![PowerShell](https://img.shields.io/badge/PowerShell-5.1+-blue)
![Windows](https://img.shields.io/badge/Platform-Windows-lightgrey)

## Features

- **Visual Port Management** - Modern WPF interface to view and control SSH port forwards
- **Auto-Reconnect** - Automatically restores dropped SSH connections
- **System Tray** - Runs minimized in the system tray
- **Configurable** - Edit host, user, and port list from the GUI
- **Status Monitoring** - Real-time status of each forwarded port

## Requirements

- Windows 10/11
- PowerShell 5.1 or later
- OpenSSH client (included in Windows 10 1809+)
- SSH key authentication configured for the remote host

## Installation

1. Clone the repository:

   ```
   git clone https://github.com/kasperp/port-manager.git
   ```

2. Ensure SSH key authentication is set up for your remote host:

   ```
   ssh-copy-id user@hostname
   ```

3. **(Optional)** Install to start automatically on Windows login:

   ```powershell
   powershell -ExecutionPolicy Bypass -File Install.ps1
   ```

   To remove from startup:

   ```powershell
   powershell -ExecutionPolicy Bypass -File Uninstall.ps1
   ```

## Usage

### Launch the Application

**Option 1: VBScript (Recommended)** - Launches completely hidden:

```
PortManager.vbs
```

**Option 2: Batch file**:

```
PortManager.bat
```

**Option 3: Direct PowerShell**:

```powershell
powershell -ExecutionPolicy Bypass -File PortManager.ps1
```

### Using the Interface

1. **Configure Connection** - Set your SSH host, username, and ports in the settings section
2. **Add Ports** - Enter a port number and click **Add Port**
3. **Start Forwarding** - Click **Start All** to establish SSH tunnels for all configured ports
4. **Monitor Status** - View real-time connection status for each port
5. **Stop Forwarding** - Click **Stop All** to terminate all SSH tunnels

### System Tray

- Minimize the window to send it to the system tray
- Double-click the tray icon to restore
- Right-click for context menu options

## Configuration

Settings are stored in `ports.json`:

```json
{
  "Host": "example.com",
  "Ports": [5001, 5000, 4096],
  "User": "username"
}
```

## How It Works

The application creates SSH tunnels using the `-L` flag for local port forwarding:

```
ssh -N -L 127.0.0.1:PORT:127.0.0.1:PORT user@host
```

Each port gets its own SSH process with keepalive settings to detect and recover from connection drops.

## License

MIT
