# ADR-003: Transform Hierarchy

## Status
Decided — **Flat BFS propagation**

## Context
Lance Engine needs parent-child transform relationships for:
- Weapon attachment to player (sword in hand, sword on back)
- Camera following player entity
- Future: skeletal animation, vehicle mounting

The hierarchy must compute world-space (global) transforms from local-space transforms relative to parents.

## Options Considered

### Recursive DFS
- Natural tree traversal: visit parent, recurse into children
- Simple to implement
- Risk of stack overflow with deep hierarchies
- Hard to parallelize

### Flat BFS (chosen)
- Collect root entities, process breadth-first
- Each level can be processed after its parent level completes
- No recursion, no stack overflow risk
- Easier to reason about ordering guarantees
- Future: each BFS level could be parallelized

### Sorted array approach (Bevy-style)
- Sort entities by depth, process in order
- Cache-friendly linear scan
- More complex bookkeeping to maintain sort order on hierarchy changes

## Decision
**Flat BFS** propagation:

1. Find all root entities (entities with `LocalTransform` but no `Parent`)
2. For roots: `GlobalTransform = LocalTransform`
3. BFS queue: add all children of roots
4. For each child: `GlobalTransform = parent.GlobalTransform * child.LocalTransform`
5. Continue until queue is empty

## Component Design

```rust
struct Parent(Entity);           // Who is my parent?
struct Children(Vec<Entity>);    // Who are my children?
struct LocalTransform(Mat4);     // Transform relative to parent
struct GlobalTransform(Mat4);    // Computed world-space transform
```

- `Parent` and `Children` are kept in sync by helper functions (`add_child`, `remove_child`)
- `GlobalTransform` is read-only from the perspective of other systems — only `transform_propagation_system` writes it
- Renderer reads `GlobalTransform` for drawing

## Consequences
- Systems that modify position must write to `LocalTransform`, not `GlobalTransform`
- `GlobalTransform` is one frame behind for newly added children (computed next frame)
- Hierarchy changes (re-parenting) must update both `Parent` and `Children` components
- Flat BFS has O(n) time complexity where n = total entities in hierarchy
- No support for non-uniform scale propagation initially (can be added later)
