# sd_tester

Native Windows SD/USB diagnostic and benchmarking tool built in Rust with `eframe/egui`.

This app is designed for hardware enthusiasts and media/forensics validation workflows where you need:

- Real write-path behavior instead of synthetic cached numbers
- Counterfeit-capacity detection patterns
- Usability verdicts from benchmark results (Pass/Caution/Fail)
- A responsive UI while long-running disk tests execute in background threads

## Highlights

- Windows 10/11 native GUI (`egui`)
- Multi-session testing across removable targets (SD + USB)
- Dedicated worker threads for disk I/O (UI stays responsive)
- Real-time throughput/latency plotting
- Benchmark assessment context:
  - ActionCam profile usability guidance
  - OS 4K random profile usability guidance
- Capacity modes:
  - Look-back checkpoint verification
  - Brute-force full write + full verify
- Raw admin-only forensics paths:
  - Physical-drive stride checks
  - CID read path (controller/driver dependent)
- Session cleanup:
  - Cancel support
  - Finished-session close (`x`)
  - Automatic test artifact deletion on finish/cancel/fail

## Safety Notice

Some tests are destructive or can stress media heavily.

- **Raw Stride Test** writes to `\\.\PhysicalDriveX` and bypasses filesystem safety.
- Use sacrificial/test media where appropriate.
- Run admin-required operations only when you explicitly intend to.

## Tech Stack

- `eframe`, `egui`, `egui_plot`
- `windows-sys` for Win32 calls
- `xxhash-rust` for deterministic high-speed data patterns
- `runas` for elevation relaunch support
- Rust std `mpsc` channels for engine-to-UI messaging

## Build and Run

### Prerequisites

- Windows 10 or 11
- Rust toolchain (stable)

### Debug

```powershell
cargo run
```

### Release

```powershell
cargo build --release
```

Executable:

`target/release/sd_tester.exe`

## GitHub Release Artifacts (EXE + MSI)

This repository includes a GitHub Actions workflow that builds Windows release artifacts and uploads them to GitHub Releases:

- Portable executable package: `sd_tester-<tag>-windows-x64.zip`
- Standalone executable: `sd_tester-<tag>-windows-x64.exe`
- Installer package: `sd_tester-<tag>-windows-x64.msi`

Workflow file:

- `.github/workflows/release-windows.yml`

How to publish a new release artifact set:

1. Create and push a semver-like tag (example `v0.1.1`):
   - `git tag v0.1.1`
   - `git push origin v0.1.1`
2. GitHub Actions builds and attaches the `.zip` and `.msi` to the release for that tag.

You can also trigger the same workflow manually with `workflow_dispatch` and a tag name.

## Test Modes

### 1) Benchmark

- **ActionCam**: sequential large-block writes, latency spike tracking.
- **OS Drive 4K Random**: random 4K writes with aligned offsets for unbuffered semantics.

Outputs include:

- Throughput and latency plots
- Peak/Average throughput
- Peak/Average latency
- Pass/Caution/Fail assessment with practical context

### 2) Capacity

- **Look-Back**: write + checkpoint verification (first and newest regions).
- **Brute-Force Full Verify**: write target capacity then verify all blocks.

Outputs include:

- Write/verify progress
- Estimated time remaining
- Tested/Verified/Usable capacity
- Counterfeit/wrap-around failure signaling

### 3) Forensics (Admin)

- Raw stride marker workflow
- CID read path via Win32 IOCTL flow (hardware/driver support varies)

## Performance Notes

- Worker threads are prioritized to reduce desktop stutter during tests.
- UI updates are throttled to avoid render-message overload.
- Capacity writes use chunked deterministic pseudo-random generation for high throughput with verifiability.

## Current Limitations

- CID retrieval depends on controller/driver behavior and may fail on many devices.
- Bus-type identification is best-effort from available storage properties.
- Some removable media/adapter stacks enforce restrictive access patterns.

## License

No license file has been added yet. Add a `LICENSE` if you plan to distribute this project publicly.
