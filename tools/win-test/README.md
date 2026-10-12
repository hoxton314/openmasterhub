# Testing masterhubd on Windows (VM)

1. On Linux: `tools/win-test/serve.sh` (builds the .exe and serves it to the VM network).
2. Start the VM: `virsh -c qemu:///system start win11` (the modules move into it).
3. In Windows PowerShell:
   ```
   iwr http://192.168.122.1:8765/run.ps1 -OutFile run.ps1; powershell -ExecutionPolicy Bypass -File .\run.ps1
   ```
4. Press keys, turn the dial and knobs, roll, move the fader; then
   `type $env:USERPROFILE\masterhub-actions.log`. The top knob changes the Windows volume.

In a VM the Base's internal hubs are not visible, so module placement is unknown there.
