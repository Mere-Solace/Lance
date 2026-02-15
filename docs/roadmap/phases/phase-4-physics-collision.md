# Phase 4: Physics & Collision

## Goals
Implement a physics system with gravity and velocity integration, a collision detection system, and a player entity with a capsule body that replaces the fly-camera.

## Related Issues
- [#3 Physics System](https://github.com/Mere-Solace/Lance/issues/3) — P1, blocked by #1
- [#4 Collision Detection](https://github.com/Mere-Solace/Lance/issues/4) — P1, blocked by #1, #3
- [#5 Player Entity with Capsule Body](https://github.com/Mere-Solace/Lance/issues/5) — P1, blocked by #2, #4

## Design Decisions
- [ADR-002: Physics Approach](../decisions/002-physics-approach.md) — Semi-implicit Euler, fixed timestep

## Acceptance Criteria

### Issue #3: Physics System
- [ ] `Velocity`, `Acceleration`, `Mass`, `GravityAffected` components
- [ ] Semi-implicit Euler integration at fixed 1/60s timestep
- [ ] Accumulator pattern handles variable frame time
- [ ] Entities with `GravityAffected` fall under gravity
- [ ] Configurable gravity constant (default -9.81 Y)

### Issue #4: Collision Detection
- [ ] `Collider` enum with Sphere, Capsule, Plane variants
- [ ] `Static` marker for immovable objects
- [ ] Sphere-Plane, Capsule-Plane, Sphere-Sphere, Capsule-Sphere detection
- [ ] Impulse-based collision response with restitution
- [ ] `CollisionEvent` emitted for each contact
- [ ] Ground plane stops entities from falling through

### Issue #5: Player Entity
- [ ] `create_capsule()` mesh generator
- [ ] Player entity with Transform, CapsuleCollider, Velocity, GravityAffected
- [ ] Camera follows player at eye height offset
- [ ] F1 toggles between player mode and debug fly-cam
- [ ] WASD moves player via velocity/forces
- [ ] Spacebar jumps when grounded
- [ ] Player slides along surfaces on collision

## Performance Budget
- Physics + collision must handle 1000 bodies at 60fps
- Target: <5ms per physics frame for the full pipeline
- Profile gate: after this phase, run 500-entity stress test
