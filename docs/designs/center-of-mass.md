# Center of Mass for Composite Entities

## Problem
Entities composed of multiple child parts (e.g., a ragdoll, a multi-part object) need
gravity and physics to act on the composite as a whole. Currently, only root entities
carry `Velocity` / `GravityAffected` / `Collider`. Child parts are visual-only and move
rigidly with their parent via the transform hierarchy.

This works for the player (upright capsule collider, no tipping). But non-player composite
entities (enemies, destructible objects, ragdolls) will need a computed center of mass so
that gravity pulls from the correct point and torque can cause rotation.

## Design

### Component: `CompositeBody`
Attached to the root entity at spawn time. Precomputed, not updated per frame (unless
parts are added/removed at runtime).

```rust
pub struct CompositeBody {
    /// Center of mass in local space, relative to root entity origin.
    pub center_of_mass: Vec3,
    /// Total mass of all parts combined.
    pub total_mass: f32,
    /// Moment of inertia tensor (diagonal approximation for phase 1).
    pub inertia: Vec3,
}
```

### Calculation at Spawn Time
When a composite entity is created:

1. Walk the child hierarchy (BFS/DFS from root).
2. For each child with a `Mass` component, read its `LocalTransform` position (the offset
   from parent) and accumulate into the center-of-mass calculation.
3. Parts without `Mass` are treated as massless (visual-only).

```
center_of_mass = sum(child_mass_i * child_local_pos_i) / total_mass
```

For nested children (e.g., forearm is child of upper arm), the position used is the
**world-relative offset** — computed by multiplying through the local transform chain
from root to child (just position, ignoring rotation/scale for the mass calculation).

### Inertia Tensor (Simplified)
For phase 1, approximate each part as a point mass at its offset from center of mass:

```
I_x = sum(m_i * (dy_i^2 + dz_i^2))
I_y = sum(m_i * (dx_i^2 + dz_i^2))
I_z = sum(m_i * (dx_i^2 + dy_i^2))
```

where `(dx, dy, dz)` is the part's position minus the center of mass.

This is sufficient for basic tipping/rotation. Full tensor (off-diagonal terms) can be
added later if needed.

### How Physics Uses It
In `physics_system`, when integrating an entity with `CompositeBody`:

- **Gravity** applies at center of mass (already correct if the force is uniform — gravity
  doesn't care about CoM for linear acceleration, only for torque).
- **Torque from gravity**: If center of mass is not directly above/below the support point
  (contact point from collision), gravity creates a torque that causes rotation.
  `torque = cross(r, m * g)` where `r` is the vector from contact point to CoM.
- **Angular velocity**: New component `AngularVelocity(Vec3)` stores rotation rate.
  Integrated each step: `rotation += angular_vel * dt` (quaternion integration).
- **Angular damping**: Similar to `Drag` but for rotation.

### What Changes for the Player
Nothing — for now. The player entity does NOT get `CompositeBody`. The player's capsule
collider is centered on the root, gravity acts on the root, and no tipping occurs. The
player stays upright via the existing system.

### When to Add `CompositeBody`
- **Enemies** with articulated bodies (future)
- **Ragdolls** (if player death or enemy death uses ragdoll physics)
- **Multi-part objects** that can tip over (e.g., a table, a stack of crates)

### Integration Plan

| Step | Description | Files |
|------|-------------|-------|
| 1 | Add `CompositeBody` component | `components/mod.rs` |
| 2 | Add `AngularVelocity` component | `components/mod.rs` |
| 3 | Write `compute_composite_body()` helper | `systems/physics.rs` or new `systems/composite.rs` |
| 4 | Update `physics_system` to apply torque for entities with `CompositeBody` | `systems/physics.rs` |
| 5 | Update collision response to compute contact-point torque | `systems/collision.rs` |

### Not Needed Yet
- Runtime recalculation (parts detaching) — defer until ragdoll/destruction
- Full 3x3 inertia tensor — diagonal approximation is fine for now
- Compound colliders (multiple collider shapes per entity) — the root's single collider
  is sufficient; child parts remain visual-only for collision purposes
