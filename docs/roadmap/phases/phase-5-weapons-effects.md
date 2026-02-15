# Phase 5: Weapons & Effects

## Goals
Implement a programmatic sword mesh, weapon attachment system, throwable balls, Bezier curve math, and sword swing effects with trail rendering.

## Related Issues
- [#6 Programmatic Sword Mesh](https://github.com/Mere-Solace/Lance/issues/6) — P2, blocked by #1
- [#7 Sword Attachment, Sheath, and Unsheath](https://github.com/Mere-Solace/Lance/issues/7) — P2, blocked by #2, #6
- [#8 Throwable Ball Entity](https://github.com/Mere-Solace/Lance/issues/8) — P2, blocked by #3, #4
- [#9 Bezier Curves and Spline System](https://github.com/Mere-Solace/Lance/issues/9) — P2, blocked by #1
- [#10 Sword Swing Effects](https://github.com/Mere-Solace/Lance/issues/10) — P3, blocked by #7, #9

## Acceptance Criteria

### Issue #6: Sword Mesh
- [ ] `create_box()` and `create_cylinder()` mesh generators
- [ ] `create_sword()` composes blade + crossguard + handle into single Mesh
- [ ] Sword renders correctly with cel shading
- [ ] Mesh generators are reusable for other objects

### Issue #7: Sword Attachment
- [ ] RightHand and Back attachment points as child entities
- [ ] Equipped / Sheathed components
- [ ] Key press toggles state, re-parents sword
- [ ] Smooth interpolation over ~0.3s during transition

### Issue #8: Throwable Balls
- [ ] Right mouse button spawns ball at camera position
- [ ] Ball has physics + collision (gravity, bounce with restitution)
- [ ] Despawn after 5s timeout or 10 max count
- [ ] Reuses existing sphere mesh

### Issue #9: Bezier Curves
- [ ] `src/math/bezier.rs` with cubic Bezier evaluate + tangent
- [ ] Catmull-Rom spline conversion
- [ ] `SplineAnimation` component for path following
- [ ] `spline_animation_system` updates entity transforms

### Issue #10: Swing Effects
- [ ] Swing state machine: Idle -> WindUp -> Swing -> Recovery -> Idle
- [ ] Swing arc follows cubic Bezier
- [ ] Trail effect: translucent quad strip from tip positions
- [ ] Hitbox active only during Swing state
- [ ] Left mouse button triggers swing

## Maintainability Budget
Adding a new weapon type should touch <3 files.
