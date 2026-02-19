# Using Windows Performance Analyzer (WPA) with `cargo run`

## 1. Install Windows Performance Toolkit

Install the Windows Performance Toolkit (part of the Windows ADK).\
This provides:

-   `wpr.exe` (Windows Performance Recorder)\
-   Windows Performance Analyzer (WPA)

------------------------------------------------------------------------

## 2. Build in Release Mode (Recommended)

In `Cargo.toml`, ensure release builds keep debug symbols:

``` toml
[profile.release]
debug = true
```

Then build:

``` powershell
cargo build --release
```

------------------------------------------------------------------------

## 3. Start Recording CPU Trace

Open an **elevated PowerShell** and run:

``` powershell
wpr -start CPU
```

------------------------------------------------------------------------

## 4. Run Your Program

Option A (via Cargo):

``` powershell
cargo run --release
```

Option B (cleaner, recommended):

``` powershell
target\release\your_game.exe
```

Let the program run long enough to capture useful data.

------------------------------------------------------------------------

## 5. Stop Recording

In the elevated PowerShell:

``` powershell
wpr -stop trace.etl
```

This creates `trace.etl`.

------------------------------------------------------------------------

## 6. Analyze in WPA

1.  Open Windows Performance Analyzer\

2.  Open `trace.etl`\

3.  Add the graph:

        CPU Usage (Sampled)

4.  Switch to:

        Call Stack

You can now inspect:

-   CPU hotspots\
-   Per-thread usage\
-   Function-level breakdown\
-   Full call stacks

------------------------------------------------------------------------

## Notes

-   Always profile `--release` builds.\
-   Avoid profiling debug builds.\
-   For clean traces, prefer running the compiled `.exe` directly
    instead of `cargo run`.
