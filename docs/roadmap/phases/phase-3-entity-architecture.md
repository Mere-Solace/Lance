# Phase 3: Entity & Scene Architecture

## Goals
Introduce an ECS foundation using `hecs` and implement a transform hierarchy. This phase transitions Lance from direct struct composition to a proper entity-component architecture.

## Related Issues
- [#1 ECS Architecture: Decide and Implement](https://github.com/Mere-Solace/Lance/issues/1) — P0, blocks everything
- [#2 Transform Hierarchy / Anchor Points](https://github.com/Mere-Solace/Lance/issues/2) — P0, blocked by #1

## Design Decisions
- [ADR-001: ECS Architecture](../decisions/001-ecs-architecture.md) — hecs
- [ADR-003: Transform Hierarchy](../decisions/003-transform-hierarchy.md) — Flat BFS

## Acceptance Criteria

### Issue #1: ECS Architecture
- [ ] `hecs` added to `Cargo.toml`
- [ ] Component module exists at `src/components/`
- [ ] `Transform`, `MeshHandle`, `Color` components defined
- [ ] Existing sphere and ground plane are entities in a `World`
- [ ] `Renderer::draw_scene` queries `World` for `(Transform, MeshHandle, Color)`
- [ ] Camera remains a standalone resource
- [ ] Application compiles and renders identically to pre-ECS state

### Issue #2: Transform Hierarchy
- [ ] `Parent`, `Children`, `LocalTransform`, `GlobalTransform` components exist
- [ ] `transform_propagation_system` computes world matrices via BFS
- [ ] Renderer uses `GlobalTransform` for drawing
- [ ] Helper functions for `add_child` / `remove_child` maintain consistency
- [ ] Test: parent entity with child entity, child moves with parent

## Architecture Notes
- Systems are free functions taking `&mut World`
- System execution order: Update -> Physics -> PostPhysics -> Render
- Each system lives in its own module under `src/systems/`
- Resources (camera, time, input) are standalone structs, not entities
