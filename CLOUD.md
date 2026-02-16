# Cloud Code Instructions for Lance Engine

## Project Overview

Lance is a 3D game engine written in Rust using SDL2 + OpenGL (raw `gl` crate), `hecs` ECS, and `glam` for math. It renders a scene with physics, grab/throw mechanics, a sword mesh, shadow mapping, dynamic lights, and demo recording.

## Build & Run

```bash
# Build
cargo build 2>&1

# Run (opens a window — will fail headless without a display)
cargo run

# Run with demo recording (records to demos/demo.mp4)
cargo run -- --record
```

## Project Structure

```
src/
  main.rs              # Entry point, ECS setup, game loop
  camera.rs            # Camera system (first/third person, orbit)
  recording.rs         # Demo recording via ffmpeg pipe
  components/          # ECS components (transform, physics, mesh, etc.)
  engine/              # Core engine (window, input, time)
  renderer/            # OpenGL rendering (shaders, shadows, lights)
  systems/             # ECS systems (physics, grab, sword, etc.)
docs/roadmap/          # Architecture decisions and phase plans
demos/                 # Recorded demo videos
```

## Key Dependencies

- `sdl2` 0.38 (bundled) — windowing, input, GL context
- `gl` 0.6 — raw OpenGL bindings
- `glam` 0.32 — vec/mat math
- `hecs` 0.10 — ECS
- `clap` 4 — CLI args

## Common Workflows

### Implementing a New Feature (from a GitHub issue)

1. **Check out your feature branch** (usually `claude/issue-<N>-<id>`):
   ```bash
   git checkout -b claude/issue-<N>-<id>
   ```

2. **Read the issue** to understand requirements:
   ```bash
   gh issue view <N>
   ```

3. **Explore relevant source files** — start from `src/main.rs` to understand how systems are registered, then look at `src/systems/` and `src/components/` for the ECS patterns.

4. **Build frequently** to catch errors early:
   ```bash
   cargo build 2>&1
   ```

5. **Commit and push**:
   ```bash
   git add <files>
   git commit -m "Description of change (#<issue>)"
   git push -u origin claude/issue-<N>-<id>
   ```

6. **Create PR** using `gh`:
   ```bash
   gh pr create --title "Short title (#<issue>)" --body "## Summary\n- what changed\n\n## Test plan\n- how to verify"
   ```

### Adding a New ECS System

1. Create `src/systems/<name>.rs`
2. Add the system function: `pub fn <name>_system(world: &mut hecs::World, ...)`
3. Register it in `src/main.rs` game loop
4. Add any new components to `src/components/`

### Adding a New Component

1. Create or edit a file in `src/components/`
2. Define a struct (no derive macros needed for hecs — just plain structs)
3. Attach to entities in `src/main.rs` or relevant system

### Shader Changes

Shaders are embedded as string constants in the renderer modules under `src/renderer/`. Edit them inline.

### Recording a Demo

```bash
cargo run -- --record
# Play the game, press Escape to stop
# Output: demos/demo.mp4
```
Requires `ffmpeg` installed on the system.

## Architecture Notes

- **Flat ECS**: no deep hierarchies. Transform parent-child uses flat BFS.
- **Orthogonal systems**: each system reads/writes its own components, minimal coupling.
- **Semi-implicit Euler** physics with friction and drag.
- **Shadow mapping**: single directional light shadow map, plus point lights.
- See `docs/roadmap/decisions/` for full architecture decision records.

## Gotchas

- This is a graphical app — `cargo run` needs a display. In headless cloud environments, only `cargo build` will succeed.
- SDL2 is bundled (compiled from source), so first build takes longer.
- No test suite yet — verify by building and (when display available) running.
