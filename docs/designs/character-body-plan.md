# Character Body Implementation Plan (Issue #35)

## Overview
Replace the single-capsule player with a 10-primitive action figure body:
- 1 tapered box (torso — new mesh type)
- 1 sphere (head)
- 4 capsules (2-part arms: upper arm + forearm, each side)
- 4 capsules (2-part legs: upper leg + lower leg, each side)
- Sword (existing, child of torso)
- Body parts are visual-only (no colliders) — root entity's capsule collider handles all physics

## Files to Modify

### 1. `src/renderer/mesh.rs` — Add `create_tapered_box`
New mesh generator: rectangle top, rectangle bottom, edges connecting corresponding vertices.
Parameters: `(top_w, top_d, bot_w, bot_d, height)`
- 6 faces, each as a quad with per-face flat normals computed from cross products
- Same vertex format as existing meshes: 6 floats (3 pos + 3 normal)

### 2. `src/components/mod.rs` — Update `CharacterBody`
```rust
pub struct CharacterBody {
    pub head: Entity,
    pub left_upper_arm: Entity,
    pub left_forearm: Entity,
    pub right_upper_arm: Entity,
    pub right_forearm: Entity,
    pub left_upper_leg: Entity,
    pub left_lower_leg: Entity,
    pub right_upper_leg: Entity,
    pub right_lower_leg: Entity,
    pub sword: Entity,
}
```

### 3. `src/systems/collision.rs` — Fix for child entities
- Change collision detection to use `GlobalTransform` for world-space positions (currently uses `LocalTransform.position` which is wrong for child entities)
- Extract position from Mat4: `global.0.w_axis.truncate()`
- In collision response: skip position correction for entities with `Parent` component (child entities' positions are driven by hierarchy, not physics)
- Velocity changes still apply normally (but child entities won't have Velocity)

### 4. `src/main.rs` — Restructure player spawning
- Change player mesh from `capsule_handle` to new `torso_handle` (tapered box)
- Keep physics collider as `Capsule { radius: 0.3, height: 1.0 }` for whole-body physics
- Spawn all body parts with hierarchy and individual colliders

## Character Proportions (Action Figure)

```
Torso (tapered box, player entity root):
  height: 0.50, shoulders: 0.36w × 0.18d, waist: 0.24w × 0.14d
  Physics collider: Capsule { radius: 0.18, height: 0.30 }
  Color: (0.6, 0.6, 0.7)

├── Head (sphere r=0.12)
│   Position: (0, 0.37, 0)
│   Collider: Sphere { radius: 0.12 }
│   Color: (0.7, 0.65, 0.6) — slightly warm/skin tone
│
├── L Upper Arm (capsule r=0.05, h=0.25)
│   Position: (-0.22, 0.02, 0), Z rotation: +0.14 rad (~8° outward)
│   Collider: Capsule { radius: 0.05, height: 0.25 }
│   └── L Forearm (capsule r=0.04, h=0.23)
│       Position: (0, -0.27, 0)
│       Collider: Capsule { radius: 0.04, height: 0.23 }
│
├── R Upper Arm (mirror of L)
│   Position: (0.22, 0.02, 0), Z rotation: -0.14 rad
│   └── R Forearm (mirror of L)
│       Position: (0, -0.27, 0)
│
├── L Upper Leg (capsule r=0.065, h=0.28)
│   Position: (-0.08, -0.39, 0)
│   Collider: Capsule { radius: 0.065, height: 0.28 }
│   └── L Lower Leg (capsule r=0.055, h=0.30)
│       Position: (0, -0.32, 0)
│       Collider: Capsule { radius: 0.055, height: 0.30 }
│
├── R Upper Leg (mirror of L)
│   Position: (0.08, -0.39, 0)
│   └── R Lower Leg (mirror of L)
│       Position: (0, -0.32, 0)
│
└── Sword (existing, same as current)
```

## Height Breakdown
- Head top: +0.49 above torso center
- Foot bottom: ~-0.96 below torso center
- Total visual height: ~1.45 units
- Physics capsule extends ±0.80, so feet slightly extend past

## Collision & Physics Approach
- Child body parts have **no `Collider`** — they are purely visual meshes in the transform hierarchy.
- The root entity (player) has a single capsule `Collider` that represents the whole body for physics.
- Gravity, velocity, and collision response all act on the root entity only.
- Child parts move rigidly with the root via `transform_propagation_system`.
- The collision system uses `GlobalTransform` for world-space positions and has `find_physics_root()`
  to redirect any child-entity collisions to the root (future-proofing for when children may get colliders).
- See `docs/designs/center-of-mass.md` for the plan to support tipping/rotation of non-player composites.

## Keybinding Note
- Sword sheath/unsheath is `F` key (not `1`)
