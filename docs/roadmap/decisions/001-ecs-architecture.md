# ADR-001: ECS Architecture

## Status
Decided — **hecs**

## Context
Lance Engine needs an entity-component-system architecture to move beyond direct struct composition in the main loop. The ECS must support:
- Spawning/despawning entities at runtime
- Querying entities by component sets
- Parent-child relationships (for transform hierarchy)
- Compatibility with our explicit system ordering (Update -> Physics -> PostPhysics -> Render)

## Options Considered

### hecs
- Minimal, no macros, no global state
- Doesn't own the main loop — we call systems ourselves
- Archetype-based storage for cache-friendly iteration
- ~2k lines of code, easy to understand and debug
- No built-in scheduler (we don't want one)

### bevy_ecs
- More features (change detection, system scheduling, resources)
- Heavier dependency, pulls in more of Bevy's ecosystem
- Has its own scheduler which conflicts with our explicit ordering
- Overkill for our needs at this stage

### shipyard
- Sparse-set based (different performance characteristics)
- More complex API with workloads
- Less community adoption than hecs/bevy_ecs

### legion
- Similar to hecs but with more features
- Maintenance has slowed
- API is more complex than needed

## Decision
**hecs** — it's the simplest option that meets our requirements. It gives us archetype storage and efficient queries without imposing any scheduling or ownership model. Systems remain free functions taking `&mut World`, called in explicit order from our main loop.

## Consequences
- Components are plain structs (no derive macros required, though `#[derive(Debug)]` is nice)
- Systems are free functions: `fn physics_system(world: &mut World, dt: f32)`
- Resources (camera, time, input) stay as standalone structs passed to systems
- No automatic change detection — systems must track their own state if needed
- If we outgrow hecs, migration to bevy_ecs is straightforward since the component model is similar
