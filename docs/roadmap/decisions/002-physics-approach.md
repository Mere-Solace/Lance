# ADR-002: Physics Approach

## Status
Decided — **Semi-implicit Euler with fixed timestep**

## Context
Lance Engine needs a physics system for gravity, velocity integration, and collision response. The system must:
- Handle gravity and basic forces
- Integrate position from velocity
- Run at a consistent rate regardless of frame rate
- Support collision detection and response
- Handle up to 1000 bodies at 60fps

## Options Considered

### Integration Method

**Explicit Euler**: `pos += vel * dt; vel += accel * dt`
- Simplest but energy-gaining (unstable for stiff systems)

**Semi-implicit Euler**: `vel += accel * dt; pos += vel * dt`
- Nearly as simple, but energy-conserving (symplectic)
- Good enough for game physics

**Verlet**: `pos_new = 2*pos - pos_old + accel * dt²`
- Better energy conservation, but awkward for velocity-dependent forces
- Harder to integrate with collision response

**RK4**: Fourth-order Runge-Kutta
- High accuracy, but 4x the cost per step
- Overkill for game physics at our scale

### Timestep

**Variable timestep**: Physics runs once per frame with frame delta
- Simple but non-deterministic, physics behaves differently at different frame rates

**Fixed timestep with accumulator**: Physics runs at fixed rate, accumulator handles frame time variance
- Deterministic, consistent behavior regardless of frame rate
- Standard approach for game physics

## Decision
**Semi-implicit Euler** with **fixed timestep accumulator** at 1/60s.

Semi-implicit Euler is the standard game physics integration: simple, stable, and fast. Fixed timestep ensures deterministic behavior.

```
accumulator += frame_dt
while accumulator >= PHYSICS_DT:
    // Semi-implicit Euler
    velocity += acceleration * PHYSICS_DT
    position += velocity * PHYSICS_DT
    accumulator -= PHYSICS_DT
```

### Collision Strategy
- **Broadphase**: Brute force O(n^2) — sufficient for <100 entities
- **Narrowphase**: Analytic shape-pair tests (sphere-plane, capsule-plane, sphere-sphere, capsule-sphere)
- **Response**: Impulse-based with restitution coefficient
- **Spatial acceleration**: Deferred until profiling shows it's needed (>100 entities)

## Consequences
- Physics is deterministic at the fixed timestep rate
- Collision shapes are limited to sphere, capsule, plane initially
- No continuous collision detection (tunneling possible at very high velocities)
- Brute force broadphase limits practical entity count, but spatial hashing can be added later
- Performance budget: physics + collision must complete within 5ms for 1000 bodies
