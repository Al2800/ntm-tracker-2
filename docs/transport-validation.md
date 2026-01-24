# Transport Validation: WSL Localhost vs Stdio

## Purpose
Validate WSL2 localhost port-forwarding reliability on Windows 10/11 and confirm the default transport (stdio) plus TCP fallback guidance.

## Scope
Test both **stdio** (via `wsl.exe` + stdin/stdout) and **TCP localhost** (daemon binding to `127.0.0.1`) across the scenarios below.

## Test Matrix
| OS | WSL Version | Firewall | VPN | Result |
| --- | --- | --- | --- | --- |
| Windows 10 | TBD | On | Off | Pending |
| Windows 10 | TBD | On | On | Pending |
| Windows 11 | TBD | On | Off | Pending |
| Windows 11 | TBD | On | On | Pending |

## Test Cases
1. **Localhost TCP – baseline**
   - Start daemon with TCP listener bound to `127.0.0.1:3847` inside WSL.
   - Connect from Windows host app.
2. **Firewall toggle**
   - Repeat baseline with Windows Firewall off/on.
3. **WSL restart**
   - `wsl --shutdown`, restart, retry connection.
4. **VPN active**
   - Enable VPN, retry connection.
5. **StdIO baseline**
   - Run stdio JSON-RPC harness via `wsl.exe` (see `docs/stdio-validation.md`).

## Commands (Template)
> Adjust paths and binary names as needed.

### TCP mode (WSL)
```bash
# inside WSL
daemon --rpc-tcp 127.0.0.1:3847
```

### Windows connection probe
```powershell
# from Windows
Test-NetConnection -ComputerName 127.0.0.1 -Port 3847
```

### StdIO mode (Windows)
```powershell
# from Windows (example)
wsl.exe -d <DistroName> -- /path/to/daemon --stdio
```

## Data to Record
- Connection success/failure per scenario
- Error messages or timeouts
- Average reconnect time
- Any dependency on VPN or firewall state

## Results (Pending)
Fill in once Windows tests are executed:

### Windows 10
- TCP baseline: Pending
- TCP + firewall off/on: Pending
- TCP after `wsl --shutdown`: Pending
- TCP with VPN: Pending
- StdIO baseline: Pending

### Windows 11
- TCP baseline: Pending
- TCP + firewall off/on: Pending
- TCP after `wsl --shutdown`: Pending
- TCP with VPN: Pending
- StdIO baseline: Pending

## Recommendation (Pending)
- Default transport: **stdio**
- TCP fallback: **opt-in only** if results confirm stability

