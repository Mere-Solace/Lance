# Lance Engine Roadmap

## Phase Overview

| Phase | Name | Status | Milestone | Issues |
|-------|------|--------|-----------|--------|
| 1 | Core Engine Foundation | Done | — | — |
| 2 | Core Rendering & Scene | Done | — | — |
| 3 | Entity & Scene Architecture | In Progress | [Phase 3](https://github.com/Mere-Solace/Lance/milestone/1) | #1, #2, #11, #12, #13, #14 |
| 4 | Physics & Collision | Planned | [Phase 4](https://github.com/Mere-Solace/Lance/milestone/2) | #3, #4, #5 |
| 5 | Weapons & Effects | Planned | [Phase 5](https://github.com/Mere-Solace/Lance/milestone/3) | #6, #7, #8, #9, #10 |

## Dependency Graph

```
#1 ECS Architecture [P0] ✅ ────┬──────────────────────────────────┐
                                │                                  │
#2 Transform Hierarchy [P0] <───┤     #3 Physics System [P1] <──── ┤
        │                       │              │                   │
        │                       │     #4 Collision Detection [P1] <┤
        │                       │         │    │                   │
#5 Player Entity [P1] <─────────┘─────────┘    │    #6 Sword Mesh [P2] <──┤
                                               │         │                │
                               #8 Throwable <──┘    #7 Sword Attach [P2]  │
                               Balls [P2]                 │               │
                                              #9 Bezier [P2] <────────────┘
                                                   │
                                         #10 Swing Effects [P3] <─── #7, #9

#11 Text Rendering [P1] <── #1 ✅
        │
        ├── #12 Debug HUD [P2]
        └── #13 Pause Screen [P2]

#14 Demo Recording [P3] <── #1 ✅
```

## Architecture Principles

- **Orthogonal systems**: each system operates on its own components, can be understood in isolation
- **Loosely coupled**: systems communicate through shared components on entities, not direct calls
- **Plug-in style**: removing a system doesn't break others
- **Trait-based interfaces**: where systems must interact, traits define boundaries
- **Performance pragmatism**: trait interfaces externally, concrete types internally for hot paths

See [decisions/](decisions/) for architecture decision records.

## Design Decisions

| # | Decision | Status |
|---|----------|--------|
| 001 | [ECS Architecture](decisions/001-ecs-architecture.md) | Decided: hecs |
| 002 | [Physics Approach](decisions/002-physics-approach.md) | Decided: Semi-implicit Euler |
| 003 | [Transform Hierarchy](decisions/003-transform-hierarchy.md) | Decided: Flat BFS |

## Phase Details

- [Phase 3: Entity Architecture](phases/phase-3-entity-architecture.md)
- [Phase 4: Physics & Collision](phases/phase-4-physics-collision.md)
- [Phase 5: Weapons & Effects](phases/phase-5-weapons-effects.md)
