# Issue #62 — Shadow Bug Fix Attempts

## Original Issues

1. **Contact shadow gap at box base**: A thin ring of lit pixels existed right at the base of
   box shadows where the box meets the floor. Ball and player shadows had no such gap.
2. **Shadow edge shimmer**: Shadow edges crawled and flickered as the camera moved, even
   when the scene was otherwise static.

---

## Attempt 1 — sampler2DShadow + PCF + texel-snapping

**Changes:**
- `cel.frag`: `sampler2D` → `sampler2DShadow`; rewrote `pcf_shadow()` to use
  `texture(shadow_map, vec3(uv, compare_z))` and hardware bilinear PCF via `GL_LINEAR`.
- `renderer/mod.rs`: `GL_NEAREST` → `GL_LINEAR` on shadow map textures; added
  `GL_COMPARE_REF_TO_TEXTURE` + `GL_LEQUAL` compare mode.
- `renderer/mod.rs` `cascade_matrix()`: added texel-snapping (whole-texel translation snap
  via `w_axis.x/y` adjustment) to eliminate sub-texel shimmer.

**Result: Shadows and lit areas were completely inverted (swapped).**

Root cause: `sampler2DShadow` with `GL_LEQUAL` returns **1.0 when lit** (reference ≤ stored
depth), not when in shadow. The PCF result was returned directly as a shadow factor, inverting
the entire shadow system.

---

## Attempt 2 — Fix inversion, lower minimum bias

**Changes:**
- `cel.frag`: Inverted PCF return: `return 1.0 - shadow / 9.0`.
- `cel.frag`: Lowered minimum bias from 0.001 → 0.0003 to tighten contact shadow gap.

**Result: Gap at box base was still visible.**

The gap persisted because the original ground was a **flat plane** whose only face (+Y normal)
was being culled by `gl::CullFace(gl::FRONT)` in the shadow pass. The floor wrote nothing to
the shadow map. At Y=0, the box bottom (a back face, rendered) and the floor were coplanar —
same shadow map depth — so with any non-zero bias the floor appeared lit.

---

## Attempt 3 — Switch to back-face culling, thick box floor

**Changes:**
- `renderer/mod.rs`: Changed shadow pass from `gl::FRONT` → `gl::BACK` culling, so the
  floor's top face (+Y) is now a front face and gets rendered into the shadow map.
- `prefabs.rs` `spawn_ground()`: Replaced flat plane with a 2-unit-thick box
  (top at Y=0, center at Y=-1) using `create_tapered_box`.
- `renderer/mesh.rs`: Removed now-unused `create_ground_plane()`.

**Result: Gap gone, but two new artifacts appeared:**
1. Acne-like dark patches at bases of all box shadows.
2. A lit ring at the base of ball/player shadows.

Cause 1 (acne): With back-face culling, box side faces are stored in the shadow map. The box
side at Y=0 and the adjacent floor at Y=0 are coplanar → depth fights → acne.

Cause 2 (lit ring): The floor bounding sphere was computed from the transform's scale columns.
The old flat-plane floor had `scale=1` with a 1000-unit mesh → bounding sphere radius ≈ 2.
As the camera moved, cascade frustum culling removed the floor from the shadow pass entirely,
breaking contact shadows camera-distance-dependently.

---

## Attempt 4 — Polygon offset + bounding sphere fix

**Changes:**
- `renderer/mod.rs`: Added `glPolygonOffset(2.0, 4.0)` in shadow pass to prevent acne.
- `prefabs.rs`: Changed ground to use a **unit (1×1×1) mesh** with
  `scale = Vec3::new(1000, 2, 1000)` so that `approx_bounding_sphere` reads the correct
  1000-unit radius from the transform columns. Fixes camera-distance frustum culling.

**Result: Neither original issue resolved; two new/worsened artifacts:**
1. Light spot at base of ball/player shadows grew larger.
2. Shadow bands crawled up the sides of boxes.

Root cause: `glPolygonOffset(2.0, 4.0)` pushes all stored shadow depths further from the
light. At the contact edge where `D_ball_surface ≈ D_floor` (touching), the pushed depth
`D_ball + offset` exceeded `D_floor`, making the floor test as lit (`D_floor - bias ≤ D_ball + offset`).
This widened the light-spot gap around any touching curved object. The box-side shadow bands
were caused by the box top (now in the shadow map via back-face culling) having its stored
depth distorted by the offset, creating false shadow comparisons on the lower box sides.

---

## Attempt 5 — Remove polygon offset

**Changes:**
- `renderer/mod.rs`: Removed `glPolygonOffset` entirely; relied on slope-scaled shader bias
  alone for acne prevention.

**Result: Still not fixed — light spot and box-side shadow bands persisted.**

Without polygon offset, the contact-edge gap reverts to the fundamental coplanar problem.
With back-face culling, the box top (front face) is in the shadow map and continues to cast
self-shadow bands on the lower box sides. The shader slope-scaled bias alone does not prevent
the contact-edge lit ring.

---

## What Was Kept

The ground is now a thick box (`create_tapered_box(1.0, 1.0, 1.0, 1.0, 1.0)` with
`scale = Vec3::new(1000, 2, 1000)`). This is retained for correct bounding sphere computation
and visual consistency. All shader and renderer changes were reverted to the pre-issue-62 state.

## Underlying Problem (Unresolved)

The contact shadow gap at box bases is a classic **coplanar geometry** problem in shadow
mapping: the box bottom face and the floor top face are both at Y=0 in world space, producing
identical depths in the shadow map. Any positive receiver-side bias makes the floor appear lit
at the contact point. Common proper solutions involve:

- Decoupled geometry (boxes not exactly coplanar with floor)
- Screen-space contact shadows
- Ray-marched soft contact shadows
- Separating caster and receiver depth layers
