# Download masterhubd.exe and the test config from the Linux host, list modules, run the service.
param([string]$Base = "http://192.168.122.1:8765")
$dir = "$env:USERPROFILE\openmasterhub-test"
New-Item -ItemType Directory -Force $dir | Out-Null
Invoke-WebRequest "$Base/masterhubd.exe" -OutFile "$dir\masterhubd.exe"
Invoke-WebRequest "$Base/config.toml" -OutFile "$dir\config.toml"
Remove-Item -ErrorAction SilentlyContinue "$env:USERPROFILE\masterhub-actions.log"
& "$dir\masterhubd.exe" --list
$env:RUST_LOG = "debug"
# Through cmd, so the service's log (stderr) is merged before PowerShell sees it; otherwise
# Windows PowerShell shows every log line as a red NativeCommandError.
cmd /c "`"$dir\masterhubd.exe`" --config `"$dir\config.toml`" 2>&1" | Tee-Object "$dir\masterhubd.log"
