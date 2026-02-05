Add-Type -AssemblyName PresentationFramework
Add-Type -AssemblyName PresentationCore
Add-Type -AssemblyName WindowsBase
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

# Hide console window
Add-Type -Name Win32 -Namespace Native -MemberDefinition @'
    [DllImport("kernel32.dll")]
    public static extern IntPtr GetConsoleWindow();
    [DllImport("user32.dll")]
    public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
'@
$consoleWindow = [Native.Win32]::GetConsoleWindow()
[void][Native.Win32]::ShowWindow($consoleWindow, 0)

# Configuration
$script:Config = @{
    Host = "46.4.77.8"
    User = "kasper"
    Ports = @(5001, 5000, 4096)
    ConfigFile = "$PSScriptRoot\ports.json"
}

# Load saved config
function Load-Config {
    if (Test-Path $script:Config.ConfigFile) {
        try {
            $saved = Get-Content $script:Config.ConfigFile | ConvertFrom-Json
            $script:Config.Ports = @($saved.Ports)
            $script:Config.Host = $saved.Host
            $script:Config.User = $saved.User
        } catch { }
    }
}

function Save-Config {
    @{
        Ports = $script:Config.Ports
        Host = $script:Config.Host
        User = $script:Config.User
    } | ConvertTo-Json | Set-Content $script:Config.ConfigFile
}

# Track SSH processes for auto-reconnect
$script:SshProcesses = @{}
$script:AutoReconnect = $true
$script:ShowPending = $false
$script:ForceQuit = $false

function Test-PortInUse {
    param([int]$Port)
    try {
        $conn = Get-NetTCPConnection -LocalPort $Port -ErrorAction SilentlyContinue
        return $null -ne $conn
    } catch { return $false }
}

function Get-PortProcess {
    param([int]$Port)
    try {
        $conn = Get-NetTCPConnection -LocalPort $Port -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($conn) {
            $proc = Get-Process -Id $conn.OwningProcess -ErrorAction SilentlyContinue
            return @{ PID = $conn.OwningProcess; Name = $proc.ProcessName; State = $conn.State }
        }
    } catch { }
    return $null
}

function Start-PortForward {
    param([int[]]$Ports)
    foreach ($port in $Ports) {
        if (-not (Test-PortInUse -Port $port)) {
            $sshArgs = @(
                "-N",
                "-o", "ServerAliveInterval=30",
                "-o", "ServerAliveCountMax=3",
                "-o", "ExitOnForwardFailure=yes",
                "-o", "StrictHostKeyChecking=accept-new",
                "-L", "127.0.0.1:${port}:127.0.0.1:${port}",
                "$($script:Config.User)@$($script:Config.Host)"
            )
            $proc = Start-Process -FilePath "ssh" -ArgumentList $sshArgs -WindowStyle Hidden -PassThru
            $script:SshProcesses[$port] = $proc.Id
        }
    }
}

function Stop-PortForwards {
    # Kill tracked SSH processes
    foreach ($procId in $script:SshProcesses.Values) {
        try {
            Stop-Process -Id $procId -Force -ErrorAction SilentlyContinue
        } catch { }
    }
    $script:SshProcesses.Clear()
    
    # Also kill any ssh.exe with our port forwards
    Get-Process -Name "ssh" -ErrorAction SilentlyContinue | ForEach-Object {
        try {
            $cmdLine = (Get-CimInstance Win32_Process -Filter "ProcessId = $($_.Id)").CommandLine
            if ($cmdLine -match "-L.*127.0.0.1") {
                Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue
            }
        } catch { }
    }
}

function Start-AutoReconnect {
    # Check and restart dropped connections
    if (-not $script:AutoReconnect) { return }
    
    $portsToRestart = @()
    foreach ($port in $script:Config.Ports) {
        if (-not (Test-PortInUse -Port $port)) {
            $portsToRestart += $port
        }
    }
    
    if ($portsToRestart.Count -gt 0) {
        Start-PortForward -Ports $portsToRestart
    }
}

Load-Config

# Create app icon (network ports symbol)
function Create-AppIcon {
    $bitmap = New-Object System.Drawing.Bitmap(32, 32)
    $g = [System.Drawing.Graphics]::FromImage($bitmap)
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.Clear([System.Drawing.Color]::Transparent)
    
    # Background circle
    $bgBrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(60, 120, 200))
    $g.FillEllipse($bgBrush, 1, 1, 30, 30)
    
    # Network symbol - three connected dots
    $whitePen = New-Object System.Drawing.Pen([System.Drawing.Color]::White, 2)
    $whiteBrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::White)
    
    # Center dot
    $g.FillEllipse($whiteBrush, 13, 13, 6, 6)
    
    # Top dot and line
    $g.DrawLine($whitePen, 16, 16, 16, 7)
    $g.FillEllipse($whiteBrush, 13, 4, 6, 6)
    
    # Bottom-left dot and line
    $g.DrawLine($whitePen, 16, 16, 8, 24)
    $g.FillEllipse($whiteBrush, 5, 21, 6, 6)
    
    # Bottom-right dot and line
    $g.DrawLine($whitePen, 16, 16, 24, 24)
    $g.FillEllipse($whiteBrush, 21, 21, 6, 6)
    
    $bgBrush.Dispose()
    $whitePen.Dispose()
    $whiteBrush.Dispose()
    $g.Dispose()
    
    return [System.Drawing.Icon]::FromHandle($bitmap.GetHicon())
}

$script:AppIcon = Create-AppIcon

# XAML UI Definition
[xml]$xaml = @"
<Window
    xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation"
    xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
    Title="Port Manager"
    Height="600" Width="580"
    WindowStartupLocation="CenterScreen"
    ResizeMode="CanResize"
    Background="White"
    MinHeight="500" MinWidth="480">

    <Window.Resources>
        <!-- Modern Button Style -->
        <Style x:Key="ModernButton" TargetType="Button">
            <Setter Property="Background" Value="#f0f0f0"/>
            <Setter Property="Foreground" Value="#1a1a1a"/>
            <Setter Property="BorderBrush" Value="#d0d0d0"/>
            <Setter Property="BorderThickness" Value="1"/>
            <Setter Property="Padding" Value="16,8"/>
            <Setter Property="FontFamily" Value="Segoe UI"/>
            <Setter Property="FontSize" Value="13"/>
            <Setter Property="Cursor" Value="Hand"/>
            <Setter Property="Template">
                <Setter.Value>
                    <ControlTemplate TargetType="Button">
                        <Border x:Name="border" Background="{TemplateBinding Background}"
                                BorderBrush="{TemplateBinding BorderBrush}"
                                BorderThickness="{TemplateBinding BorderThickness}"
                                CornerRadius="4" Padding="{TemplateBinding Padding}">
                            <ContentPresenter HorizontalAlignment="Center" VerticalAlignment="Center"/>
                        </Border>
                        <ControlTemplate.Triggers>
                            <Trigger Property="IsMouseOver" Value="True">
                                <Setter TargetName="border" Property="Background" Value="#e5e5e5"/>
                                <Setter TargetName="border" Property="BorderBrush" Value="#0078D4"/>
                            </Trigger>
                            <Trigger Property="IsPressed" Value="True">
                                <Setter TargetName="border" Property="Background" Value="#d0d0d0"/>
                            </Trigger>
                        </ControlTemplate.Triggers>
                    </ControlTemplate>
                </Setter.Value>
            </Setter>
        </Style>

        <!-- Modern TextBox Style -->
        <Style x:Key="ModernTextBox" TargetType="TextBox">
            <Setter Property="Background" Value="White"/>
            <Setter Property="Foreground" Value="#1a1a1a"/>
            <Setter Property="BorderBrush" Value="#d0d0d0"/>
            <Setter Property="BorderThickness" Value="1"/>
            <Setter Property="Padding" Value="8,6"/>
            <Setter Property="FontFamily" Value="Segoe UI"/>
            <Setter Property="FontSize" Value="13"/>
            <Setter Property="Template">
                <Setter.Value>
                    <ControlTemplate TargetType="TextBox">
                        <Border x:Name="border" Background="{TemplateBinding Background}"
                                BorderBrush="{TemplateBinding BorderBrush}"
                                BorderThickness="{TemplateBinding BorderThickness}"
                                CornerRadius="4">
                            <ScrollViewer x:Name="PART_ContentHost" Margin="{TemplateBinding Padding}"/>
                        </Border>
                        <ControlTemplate.Triggers>
                            <Trigger Property="IsFocused" Value="True">
                                <Setter TargetName="border" Property="BorderBrush" Value="#0078D4"/>
                            </Trigger>
                        </ControlTemplate.Triggers>
                    </ControlTemplate>
                </Setter.Value>
            </Setter>
        </Style>

        <!-- ListBox Styles -->
        <Style TargetType="ListBox">
            <Setter Property="Background" Value="White"/>
            <Setter Property="BorderThickness" Value="0"/>
        </Style>

        <Style TargetType="ListBoxItem">
            <Setter Property="HorizontalContentAlignment" Value="Stretch"/>
            <Setter Property="Padding" Value="0"/>
            <Setter Property="Margin" Value="0"/>
            <Style.Triggers>
                <Trigger Property="IsMouseOver" Value="True">
                    <Setter Property="Background" Value="#f0f0f0"/>
                </Trigger>
                <Trigger Property="IsSelected" Value="True">
                    <Setter Property="Background" Value="#e5f1fb"/>
                </Trigger>
            </Style.Triggers>
        </Style>
    </Window.Resources>

    <Grid Margin="20">
        <Grid.RowDefinitions>
            <RowDefinition Height="Auto"/>
            <RowDefinition Height="Auto"/>
            <RowDefinition Height="*"/>
            <RowDefinition Height="Auto"/>
            <RowDefinition Height="Auto"/>
        </Grid.RowDefinitions>

        <!-- Header -->
        <StackPanel Grid.Row="0" Margin="0,0,0,16">
            <TextBlock Text="Port Manager" FontSize="24" FontWeight="SemiBold" Foreground="#1a1a1a" FontFamily="Segoe UI"/>
            <StackPanel Orientation="Horizontal">
                <TextBlock x:Name="ConnectionStatus" Text="Ready" FontSize="12" Foreground="#666666" Margin="0,4,0,0"/>
                <TextBlock Text=" | " FontSize="12" Foreground="#cccccc" Margin="0,4,0,0"/>
                <CheckBox x:Name="AutoReconnectCheck" Content="Auto-reconnect" IsChecked="True" FontSize="12" Foreground="#666666" VerticalAlignment="Center" Margin="0,4,0,0"/>
            </StackPanel>
        </StackPanel>

        <!-- Connection Settings -->
        <Border Grid.Row="1" BorderBrush="#e0e0e0" BorderThickness="1" CornerRadius="4" Padding="16" Margin="0,0,0,16" Background="#fafafa">
            <Grid>
                <Grid.ColumnDefinitions>
                    <ColumnDefinition Width="*"/>
                    <ColumnDefinition Width="*"/>
                    <ColumnDefinition Width="Auto"/>
                </Grid.ColumnDefinitions>
                <Grid.RowDefinitions>
                    <RowDefinition Height="Auto"/>
                    <RowDefinition Height="Auto"/>
                </Grid.RowDefinitions>

                <TextBlock Text="Host" FontSize="12" Foreground="#666666" Margin="0,0,8,4" Grid.Column="0" Grid.Row="0"/>
                <TextBox x:Name="HostInput" Style="{StaticResource ModernTextBox}" Grid.Column="0" Grid.Row="1" Margin="0,0,8,0"/>

                <TextBlock Text="User" FontSize="12" Foreground="#666666" Margin="8,0,8,4" Grid.Column="1" Grid.Row="0"/>
                <TextBox x:Name="UserInput" Style="{StaticResource ModernTextBox}" Grid.Column="1" Grid.Row="1" Margin="8,0,8,0"/>

                <Button x:Name="SaveBtn" Grid.Column="2" Grid.Row="1" Style="{StaticResource ModernButton}" Padding="12,8" ToolTip="Save Settings">
                    <StackPanel Orientation="Horizontal">
                        <TextBlock Text="&#xE74E;" FontFamily="Segoe MDL2 Assets" FontSize="14" VerticalAlignment="Center"/>
                        <TextBlock Text="Save" Margin="6,0,0,0" VerticalAlignment="Center"/>
                    </StackPanel>
                </Button>
            </Grid>
        </Border>

        <!-- Ports List -->
        <Grid Grid.Row="2">
            <Grid.RowDefinitions>
                <RowDefinition Height="Auto"/>
                <RowDefinition Height="*"/>
                <RowDefinition Height="Auto"/>
            </Grid.RowDefinitions>

            <TextBlock Text="Managed Ports" FontSize="14" FontWeight="SemiBold" Foreground="#1a1a1a" Margin="0,0,0,8"/>

            <Grid Grid.Row="1" Margin="0,0,0,12">
                <Grid.ColumnDefinitions>
                    <ColumnDefinition Width="*"/>
                    <ColumnDefinition Width="Auto"/>
                </Grid.ColumnDefinitions>
                
                <Border BorderBrush="#d0d0d0" BorderThickness="1" CornerRadius="4">
                    <Grid>
                        <Grid.RowDefinitions>
                            <RowDefinition Height="Auto"/>
                            <RowDefinition Height="*"/>
                        </Grid.RowDefinitions>
                        
                        <!-- Header -->
                        <Border Background="#f5f5f5" BorderBrush="#e0e0e0" BorderThickness="0,0,0,1" Padding="12,8">
                            <Grid>
                                <Grid.ColumnDefinitions>
                                    <ColumnDefinition Width="70"/>
                                    <ColumnDefinition Width="100"/>
                                    <ColumnDefinition Width="*"/>
                                    <ColumnDefinition Width="70"/>
                                </Grid.ColumnDefinitions>
                                <TextBlock Text="Port" FontWeight="SemiBold" Foreground="#666666" FontSize="12"/>
                                <TextBlock Grid.Column="1" Text="Status" FontWeight="SemiBold" Foreground="#666666" FontSize="12"/>
                                <TextBlock Grid.Column="2" Text="Process" FontWeight="SemiBold" Foreground="#666666" FontSize="12"/>
                                <TextBlock Grid.Column="3" Text="PID" FontWeight="SemiBold" Foreground="#666666" FontSize="12"/>
                            </Grid>
                        </Border>
                        
                        <!-- List -->
                        <ListBox x:Name="PortsList" Grid.Row="1" BorderThickness="0" Background="White" Padding="0"/>
                    </Grid>
                </Border>
                
                <!-- Remove/Refresh buttons next to list -->
                <StackPanel Grid.Column="1" Margin="8,0,0,0" VerticalAlignment="Top">
                    <Button x:Name="RemoveBtn" ToolTip="Remove Selected" Style="{StaticResource ModernButton}" Padding="10" Width="38" Height="38">
                        <TextBlock Text="&#xE74D;" FontFamily="Segoe MDL2 Assets" FontSize="14"/>
                    </Button>
                    <Button x:Name="RefreshBtn" ToolTip="Refresh" Style="{StaticResource ModernButton}" Margin="0,6,0,0" Padding="10" Width="38" Height="38">
                        <TextBlock Text="&#xE72C;" FontFamily="Segoe MDL2 Assets" FontSize="14"/>
                    </Button>
                </StackPanel>
            </Grid>

            <Grid Grid.Row="2">
                <Grid.ColumnDefinitions>
                    <ColumnDefinition Width="*"/>
                    <ColumnDefinition Width="Auto"/>
                </Grid.ColumnDefinitions>

                <TextBox x:Name="AddPortInput" Style="{StaticResource ModernTextBox}" Margin="0,0,8,0"/>
                <Button x:Name="AddBtn" Grid.Column="1" Style="{StaticResource ModernButton}" Padding="12,8" ToolTip="Add Port">
                    <StackPanel Orientation="Horizontal">
                        <TextBlock Text="&#xE710;" FontFamily="Segoe MDL2 Assets" FontSize="14" VerticalAlignment="Center"/>
                        <TextBlock Text="Add" Margin="6,0,0,0" VerticalAlignment="Center"/>
                    </StackPanel>
                </Button>
            </Grid>
        </Grid>

        <!-- Action Buttons -->
        <Grid Grid.Row="3" Margin="0,16,0,0">
            <Grid.ColumnDefinitions>
                <ColumnDefinition Width="*"/>
                <ColumnDefinition Width="*"/>
            </Grid.ColumnDefinitions>

            <Button x:Name="StartBtn" Style="{StaticResource ModernButton}" Margin="0,0,6,0" Padding="16,10" ToolTip="Start All Port Forwards">
                <StackPanel Orientation="Horizontal">
                    <TextBlock Text="&#xE768;" FontFamily="Segoe MDL2 Assets" FontSize="16" VerticalAlignment="Center"/>
                    <TextBlock Text="Start All" Margin="8,0,0,0" VerticalAlignment="Center"/>
                </StackPanel>
            </Button>
            <Button x:Name="StopBtn" Grid.Column="1" Style="{StaticResource ModernButton}" Margin="6,0,0,0" Padding="16,10" ToolTip="Stop All Port Forwards">
                <StackPanel Orientation="Horizontal">
                    <TextBlock Text="&#xE71A;" FontFamily="Segoe MDL2 Assets" FontSize="16" VerticalAlignment="Center"/>
                    <TextBlock Text="Stop All" Margin="8,0,0,0" VerticalAlignment="Center"/>
                </StackPanel>
            </Button>
        </Grid>

        <!-- Status Bar -->
        <TextBlock x:Name="StatusText" Grid.Row="4" Text="Ready" FontSize="12" Foreground="#888888" Margin="0,12,0,0"/>
    </Grid>
</Window>
"@

# Create Window
$reader = New-Object System.Xml.XmlNodeReader $xaml
$window = [Windows.Markup.XamlReader]::Load($reader)

# Set window icon
$window.Icon = [System.Windows.Interop.Imaging]::CreateBitmapSourceFromHIcon(
    $script:AppIcon.Handle,
    [System.Windows.Int32Rect]::Empty,
    [System.Windows.Media.Imaging.BitmapSizeOptions]::FromEmptyOptions()
)

# Get Controls
$hostInput = $window.FindName("HostInput")
$userInput = $window.FindName("UserInput")
$saveBtn = $window.FindName("SaveBtn")
$connectionStatus = $window.FindName("ConnectionStatus")
$autoReconnectCheck = $window.FindName("AutoReconnectCheck")
$portsList = $window.FindName("PortsList")
$addPortInput = $window.FindName("AddPortInput")
$addBtn = $window.FindName("AddBtn")
$removeBtn = $window.FindName("RemoveBtn")
$refreshBtn = $window.FindName("RefreshBtn")
$startBtn = $window.FindName("StartBtn")
$stopBtn = $window.FindName("StopBtn")
$statusText = $window.FindName("StatusText")

# Set initial values
$hostInput.Text = $script:Config.Host
$userInput.Text = $script:Config.User

function Update-PortList {
    $portsList.Items.Clear()
    $activeCount = 0
    $totalCount = $script:Config.Ports.Count
    
    foreach ($port in $script:Config.Ports) {
        $item = New-Object System.Windows.Controls.ListBoxItem
        $item.Padding = "12,10"
        $item.BorderBrush = [System.Windows.Media.BrushConverter]::new().ConvertFrom("#e8e8e8")
        $item.BorderThickness = "0,0,0,1"
        
        $grid = New-Object System.Windows.Controls.Grid
        $col1 = New-Object System.Windows.Controls.ColumnDefinition
        $col1.Width = 70
        $col2 = New-Object System.Windows.Controls.ColumnDefinition
        $col2.Width = 100
        $col3 = New-Object System.Windows.Controls.ColumnDefinition
        $col3.Width = "*"
        $col4 = New-Object System.Windows.Controls.ColumnDefinition
        $col4.Width = 70
        [void]$grid.ColumnDefinitions.Add($col1)
        [void]$grid.ColumnDefinitions.Add($col2)
        [void]$grid.ColumnDefinitions.Add($col3)
        [void]$grid.ColumnDefinitions.Add($col4)
        
        # Port
        $portText = New-Object System.Windows.Controls.TextBlock
        $portText.Text = $port.ToString()
        $portText.VerticalAlignment = "Center"
        $portText.Foreground = [System.Windows.Media.Brushes]::Black
        [System.Windows.Controls.Grid]::SetColumn($portText, 0)
        [void]$grid.Children.Add($portText)
        
        # Status with dot
        $statusPanel = New-Object System.Windows.Controls.StackPanel
        $statusPanel.Orientation = "Horizontal"
        $statusPanel.VerticalAlignment = "Center"
        [System.Windows.Controls.Grid]::SetColumn($statusPanel, 1)
        
        $dot = New-Object System.Windows.Shapes.Ellipse
        $dot.Width = 8
        $dot.Height = 8
        $dot.Margin = "0,0,6,0"
        
        $statusText = New-Object System.Windows.Controls.TextBlock
        
        # Process
        $processText = New-Object System.Windows.Controls.TextBlock
        $processText.VerticalAlignment = "Center"
        $processText.Foreground = [System.Windows.Media.Brushes]::Black
        [System.Windows.Controls.Grid]::SetColumn($processText, 2)
        
        # PID
        $pidText = New-Object System.Windows.Controls.TextBlock
        $pidText.VerticalAlignment = "Center"
        $pidText.Foreground = [System.Windows.Media.Brushes]::Black
        [System.Windows.Controls.Grid]::SetColumn($pidText, 3)
        
        if ($script:ShowPending) {
            $dot.Fill = [System.Windows.Media.Brushes]::Orange
            $statusText.Text = "Pending"
            $statusText.Foreground = [System.Windows.Media.Brushes]::Orange
            $processText.Text = "-"
            $pidText.Text = "-"
        } elseif (Test-PortInUse -Port $port) {
            $procInfo = Get-PortProcess -Port $port
            $dot.Fill = [System.Windows.Media.Brushes]::LimeGreen
            $statusText.Text = "Active"
            $statusText.Foreground = [System.Windows.Media.Brushes]::Green
            $processText.Text = $procInfo.Name
            $pidText.Text = $procInfo.PID.ToString()
            $activeCount++
        } else {
            $dot.Fill = [System.Windows.Media.Brushes]::Tomato
            $statusText.Text = "Inactive"
            $statusText.Foreground = [System.Windows.Media.Brushes]::Gray
            $processText.Text = "-"
            $pidText.Text = "-"
        }
        
        [void]$statusPanel.Children.Add($dot)
        [void]$statusPanel.Children.Add($statusText)
        [void]$grid.Children.Add($statusPanel)
        [void]$grid.Children.Add($processText)
        [void]$grid.Children.Add($pidText)
        
        $item.Content = $grid
        $item.Tag = $port
        [void]$portsList.Items.Add($item)
    }
    
    # Update tray icon color based on status
    if ($script:ShowPending) {
        Update-TrayIcon -Status "pending"
    } elseif ($totalCount -eq 0) {
        Update-TrayIcon -Status "inactive"
    } elseif ($activeCount -eq $totalCount) {
        Update-TrayIcon -Status "active"
    } elseif ($activeCount -gt 0) {
        Update-TrayIcon -Status "partial"
    } else {
        Update-TrayIcon -Status "inactive"
    }
}

function Update-ConnectionStatus {
    if ($script:ShowPending) {
        $connectionStatus.Text = "Connecting..."
        $connectionStatus.Foreground = [System.Windows.Media.Brushes]::Orange
        return
    }
    
    $activeCount = 0
    $totalCount = $script:Config.Ports.Count
    
    foreach ($port in $script:Config.Ports) {
        if (Test-PortInUse -Port $port) { $activeCount++ }
    }
    
    if ($totalCount -eq 0) {
        $connectionStatus.Text = "No ports configured"
        $connectionStatus.Foreground = [System.Windows.Media.Brushes]::Gray
    } elseif ($activeCount -eq $totalCount) {
        $connectionStatus.Text = "All $totalCount ports active"
        $connectionStatus.Foreground = [System.Windows.Media.Brushes]::Green
    } elseif ($activeCount -gt 0) {
        $connectionStatus.Text = "$activeCount of $totalCount ports active"
        $connectionStatus.Foreground = [System.Windows.Media.Brushes]::Orange
    } else {
        $connectionStatus.Text = "No ports active"
        $connectionStatus.Foreground = [System.Windows.Media.Brushes]::Gray
    }
}

$autoReconnectCheck.Add_Checked({ $script:AutoReconnect = $true })
$autoReconnectCheck.Add_Unchecked({ $script:AutoReconnect = $false })

# Event Handlers
$saveBtn.Add_Click({
    $script:Config.Host = $hostInput.Text
    $script:Config.User = $userInput.Text
    Save-Config
    $statusText.Text = "Settings saved!"
})

$addBtn.Add_Click({
    $portText = $addPortInput.Text.Trim()
    if ($portText -match '^\d+$') {
        $port = [int]$portText
        if ($port -gt 0 -and $port -lt 65536) {
            if ($port -notin $script:Config.Ports) {
                $script:Config.Ports += $port
                Save-Config
                Update-PortList
                $addPortInput.Text = ""
                $statusText.Text = "Added port $port"
            } else { $statusText.Text = "Port $port already exists" }
        } else { $statusText.Text = "Invalid port number" }
    } else { $statusText.Text = "Enter a valid port number" }
})

$removeBtn.Add_Click({
    if ($portsList.SelectedItem) {
        $port = [int]$portsList.SelectedItem.Tag
        $script:Config.Ports = @($script:Config.Ports | Where-Object { $_ -ne $port })
        Save-Config
        Update-PortList
        $statusText.Text = "Removed port $port"
    } else { $statusText.Text = "Select a port to remove" }
})

$refreshBtn.Add_Click({
    Update-PortList
    Update-ConnectionStatus
    $statusText.Text = "Refreshed at $(Get-Date -Format 'HH:mm:ss')"
})

$startBtn.Add_Click({
    $statusText.Text = "Starting port forwards..."
    Start-PortForward -Ports $script:Config.Ports
    Start-Sleep -Seconds 2
    Update-PortList
    $statusText.Text = "Port forwards started!"
})

$stopBtn.Add_Click({
    $statusText.Text = "Stopping port forwards..."
    Stop-PortForwards
    Start-Sleep -Seconds 1
    Update-PortList
    $statusText.Text = "Port forwards stopped"
})

# Timer for auto-refresh
$timer = New-Object System.Windows.Threading.DispatcherTimer
$timer.Interval = [TimeSpan]::FromSeconds(10)
$timer.Add_Tick({
    Update-PortList
    Update-ConnectionStatus
    Start-AutoReconnect
})
$timer.Start()

# System Tray Icon
$script:notifyIcon = New-Object System.Windows.Forms.NotifyIcon
$script:notifyIcon.Text = "Port Manager"
$script:notifyIcon.Visible = $true

# Function to create tray icon with specific color
function Update-TrayIcon {
    param([string]$Status)  # "active", "inactive", "partial"
    
    $bitmap = New-Object System.Drawing.Bitmap(32, 32)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $graphics.Clear([System.Drawing.Color]::Transparent)
    
    switch ($Status) {
        "active" { 
            $outerColor = [System.Drawing.Color]::FromArgb(16, 124, 16)  # Green
            $script:notifyIcon.Text = "Port Manager - All Active"
        }
        "partial" { 
            $outerColor = [System.Drawing.Color]::FromArgb(255, 140, 0)  # Orange
            $script:notifyIcon.Text = "Port Manager - Partial"
        }
        "pending" { 
            $outerColor = [System.Drawing.Color]::FromArgb(255, 200, 0)  # Yellow/Orange
            $script:notifyIcon.Text = "Port Manager - Connecting..."
        }
        default { 
            $outerColor = [System.Drawing.Color]::FromArgb(200, 50, 50)  # Red
            $script:notifyIcon.Text = "Port Manager - Inactive"
        }
    }
    
    $brush = New-Object System.Drawing.SolidBrush($outerColor)
    $graphics.FillEllipse($brush, 2, 2, 28, 28)
    $whiteBrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::White)
    $graphics.FillEllipse($whiteBrush, 10, 10, 12, 12)
    $brush.Dispose()
    $whiteBrush.Dispose()
    $graphics.Dispose()
    
    $newIcon = [System.Drawing.Icon]::FromHandle($bitmap.GetHicon())
    $script:notifyIcon.Icon = $newIcon
    $bitmap.Dispose()
}

# Initial icon
Update-TrayIcon -Status "pending"

# Context menu for tray icon
$script:contextMenu = New-Object System.Windows.Forms.ContextMenuStrip

function Update-TrayMenu {
    $script:contextMenu.Items.Clear()
    
    # Port status items
    foreach ($port in $script:Config.Ports) {
        $portItem = New-Object System.Windows.Forms.ToolStripMenuItem
        if (Test-PortInUse -Port $port) {
            $portItem.Text = "[*] Port $port - Active"
            $portItem.ForeColor = [System.Drawing.Color]::Green
        } else {
            $portItem.Text = "[ ] Port $port - Inactive"
            $portItem.ForeColor = [System.Drawing.Color]::Gray
        }
        $portItem.Enabled = $false
        [void]$script:contextMenu.Items.Add($portItem)
    }
    
    [void]$script:contextMenu.Items.Add("-")
    
    $showItem = New-Object System.Windows.Forms.ToolStripMenuItem
    $showItem.Text = "Show Port Manager"
    $showItem.Add_Click({
        $window.Show()
        $window.WindowState = "Normal"
        $window.Activate()
    })
    [void]$script:contextMenu.Items.Add($showItem)
    
    [void]$script:contextMenu.Items.Add("-")
    
    $startItem = New-Object System.Windows.Forms.ToolStripMenuItem
    $startItem.Text = "Start All Forwards"
    $startItem.Add_Click({
        Start-PortForward -Ports $script:Config.Ports
    })
    [void]$script:contextMenu.Items.Add($startItem)
    
    $stopItem = New-Object System.Windows.Forms.ToolStripMenuItem
    $stopItem.Text = "Stop All Forwards"
    $stopItem.Add_Click({
        Stop-PortForwards
    })
    [void]$script:contextMenu.Items.Add($stopItem)
    
    [void]$script:contextMenu.Items.Add("-")
    
    $exitItem = New-Object System.Windows.Forms.ToolStripMenuItem
    $exitItem.Text = "Quit"
    $exitItem.Add_Click({
        $script:ForceQuit = $true
        $script:notifyIcon.Visible = $false
        $script:notifyIcon.Dispose()
        Stop-PortForwards
        $window.Close()
    })
    [void]$script:contextMenu.Items.Add($exitItem)
}

Update-TrayMenu
$script:notifyIcon.ContextMenuStrip = $script:contextMenu

# Refresh menu when opening
$script:contextMenu.Add_Opening({ Update-TrayMenu })

# Double-click tray icon to show window
$script:notifyIcon.Add_DoubleClick({
    $window.Show()
    $window.WindowState = "Normal"
    $window.Activate()
})

# Minimize to tray instead of taskbar
$window.Add_StateChanged({
    if ($window.WindowState -eq "Minimized") {
        $window.ShowInTaskbar = $false
        $window.Hide()
    } else {
        $window.ShowInTaskbar = $true
    }
})

# Handle window closing - hide to tray instead (unless force quit)
$window.add_Closing([System.ComponentModel.CancelEventHandler]{
    param($sender, $e)
    if (-not $script:ForceQuit) {
        $e.Cancel = $true
        $window.Hide()
        $window.ShowInTaskbar = $false
    }
})

# Initial load - show pending and start ports
$script:ShowPending = $true
Update-PortList
Update-ConnectionStatus
$statusText.Text = "Starting port forwards..."

# Start port forwards in background, then refresh
$window.Add_ContentRendered({
    Start-PortForward -Ports $script:Config.Ports
    Start-Sleep -Seconds 2
    $script:ShowPending = $false
    Update-PortList
    Update-ConnectionStatus
    $statusText.Text = "Port forwards started!"
})

# Show window
$window.ShowDialog() | Out-Null

# Cleanup
$timer.Stop()
$script:notifyIcon.Visible = $false
$script:notifyIcon.Dispose()
