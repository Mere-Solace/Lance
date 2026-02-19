# Combat System Design Document

## Core Philosophy

**"Physics-Driven Combat Sandbox with Skill Expression"**

- Every object in the world is a potential weapon or tool
- Skill ceiling based on mastery, not stat grinding
- Aggressive play rewarded, defensive play sustainable but limiting
- Emergent gameplay through consistent physics rules

---

## World Interactivity

### Destruction Model: Hybrid Approach

**Destructible Objects:**
- Pillars, walls, crates, statues, furniture
- Sword slashes can shatter wooden posts (splinters become physics objects)
- Players can kick/throw rubble at enemies
- Environmental objects are tactical resources

**Terrain Deformation:**
- Surface-level damage only (sword gashes, fireball craters)
- Affects movement and cover
- Regenerates over time
- **NOT** fully mineable - maintains combat focus

**Design Rationale:**
Full voxel destruction would shift focus from combat mastery to world manipulation. Destructible props provide interactivity while keeping the Umbral Blade system central.

---

## The Umbral Blade System

### Core Concept
A floating sword that the player controls remotely. The primary weapon and skill expression tool.

### Blade States

1. **SHEATHED**
   - Hovers behind player (default position)
   - Passive Soulfire regeneration (slow)
   - Can quick-draw into any attack
   - Safest state, lowest DPS

2. **ACTIVE/READY**
   - Orbits player at medium distance
   - Instant command execution
   - No regen, no drain
   - Neutral combat state

3. **WIELDED**
   - Player physically holds blade
   - Traditional melee attacks
   - Can combo into remote commands
   - Blocks Soulfire regen while held

4. **ATTACKING**
   - Executing a command (slash, lunge, spiral)
   - Cannot be interrupted (commitment mechanic)
   - Vulnerable if missed
   - Higher cost = longer recovery

5. **IMPALED/ANCHORED**
   - Stuck in enemy or environment
   - Drains impaled target slowly
   - Creates dash tether point
   - Can be left while using found weapons

6. **WAITING/POSITIONED**
   - Sent to specific location to hold
   - Can guard areas, create zones, prepare ambushes
   - Small Soulfire upkeep per second
   - PvP pressure tool

### Core Mechanics

#### Dash-To-Equip System
When dashing to blade, attack varies by input:

- **Neutral dash** → Grab + overhead slam
- **Dash + direction** → Grab + spinning slash
- **Dash + jump** → Aerial reclaim + downward spike
- **Dash + attack** → Kick blade toward enemy (redirect without grabbing)
- **Late dash** → Desperate catch + risky high-damage thrust

#### Impalement System (Signature Mechanic)

**While Impaled:**
- **Hold position** → Drain (1% health/sec → Soulfire conversion)
- **Twist** → Increased damage, enemy staggers, blade releases
- **Retract** → Pull enemy toward extraction point
- **Push through** → Exit opposite side, enemy staggers forward (costs Soulfire)
- **Anchor & dash** → Leave blade in, create tether that yanks enemy on landing

**Extraction Physics:**
Direction and force of extraction creates opportunities:

- **Straight out** → Enemy stumbles backward
- **Upward** → Launch into air (juggle setup)
- **Sideways** → Spin enemy, create opening
- **Explosive** (Soulfire cost) → Knockback + bleed DoT

**Multi-Target Impale:**
Blade can impale multiple enemies in sequence, creating combo opportunities where all impaled targets are tethered together.

---

## Soulfire Economy

### The Resource System

**Purpose:** Universal currency for mobility, utility, and offensive options. Creates risk/reward around defensive play.

### Gain Soulfire

- Landing hits (spell-vamp style)
- Bare-hand parrying (high risk = high reward)
- Environmental kills (impaling on spikes, collapsing pillars)
- Draining impaled enemies

### Spend Soulfire

- Dashing
- High jumps
- Grabbing heavy objects/enemies
- Casting blade spells
- Weapon recall (when thrown blade is distant)
- Advanced blade commands (push-through, explosive extract)

### Lose Soulfire

- Taking damage (even blocked damage)
- Holding shield too long (passive drain)
- Missing high-commitment attacks
- Waiting/Positioned blade upkeep

### Design Impact

**Shield Paradox:** Blocking is safe but starves you of resources. Turtling is viable short-term but unsustainable long-term.

**Aggressive Loop:** Hit → gain Soulfire → unlock mobility → better positioning → more hits → loop

**Skill Expression:** Experts maintain high Soulfire through constant offense. Beginners drain resources by over-defending.

---

## Combat Flow

### Intended Loop

1. Umbral Blade combos build Soulfire
2. Soulfire enables movement/utility/spells
3. Movement creates positioning for more combos
4. Complexity increases as Soulfire pool grows

### Example Emergent Combo

```
Throw blade (impale Enemy A)
→ Dash to impaled blade
→ Pull out mid-dash (extract upward)
→ Enemy A launches into air
→ Spin attack juggle
→ Enemy A's sword drops
→ Grab sword mid-air
→ Dual-wield finish on Enemy A
→ Throw both swords at Enemy B
→ Recall Umbral Blade with Soulfire
→ Repeat
```

---

## Grab & Throw System

### Tiered Interaction

**Tier 1 - Always Grabbable:**
- Weapons (swords, shields, arrows)
- Environmental props (rocks, bottles, debris)
- Spell projectiles mid-flight (high skill, high reward)

**Tier 2 - Contextual Grabs:**
- Stunned/ragdolled humanoid enemies
- Small creatures (bugs, rats - they escape after thrown)
- Heavy objects (requires Soulfire expenditure)

**Tier 3 - Magic-Augmented Only:**
- Large enemies (bears, bosses - requires Soulfire + possible upgrade)
- Players in PvP (only when knocked down, mid-air, or stunned)

### Core Rule

**"If it has physics, it's interactible."**

Effectiveness scales with size and resistance. This maintains emergence without absurdity.

---

## Weapon & Combat Systems

### Physical Projectiles

- All attacks/weapons/items exist as physical objects in worldspace
- Projectiles collide mid-air (can block each other)
- Thrown swords impale enemies and can be dashed to
- Arrows can be picked up and reused

### Found Weapons

- Can be picked up and used alongside Umbral Blade
- Degrade/break over time (encourages creative use, not hoarding)
- Each weapon type has unique properties
- Throwing weapons creates environmental hazards

### Bare-Handed Combat

- Can grab swords out of the air
- Parrying without weapons grants high Soulfire
- Can perform grabs/throws
- High risk, high reward playstyle option

### Shields

- Block damage but drain Soulfire when held
- Overuse prevents spell casting
- Getting hit (even blocked) allows enemy to drain your Soulfire
- Creates decision: immediate safety vs long-term resources

---

## PvP Specific Mechanics

### Blade Interactions

**Blade vs Blade Collision:**
Both blades clash and return to owners (neutral reset). Creates mindgames around simultaneous attacks.

**Stealing Opponent's Blade:**
If enemy blade is impaled in environment, can be grabbed (counts as heavy object, costs Soulfire). Forces opponent to recall or fight with found weapons.

**Parrying Umbral Blade:**
- Bare hands: High Soulfire reward, blade bounces to random position
- Shield: Safe but drains Soulfire, blade returns to owner

### Environmental Zoning

Impale blade into wall/pillar to create attack zone. Opponent must destroy object or bait the attack.

### Balance Mechanics

- Soulfire starts at 50% for both players (prevents spawn advantage)
- Environmental kills grant bonus Soulfire
- Arenas have consistent destructible prop placement

---

## Roguelike Structure

### Per-Run Variation

**Environmental Diversity:**
- Different destructible layouts per room
- Biome-specific hazards (ice palace = breakable icicles, forge = molten metal)
- Found weapons vary by location

**Tactical Adaptation:**
- Library: Throwable books, collapsible shelves
- Forge: Molten hazards, heavy anvils to drop
- Crypt: Brittle pillars, bone piles for projectiles

### Progression Systems

**Permanent Unlocks:**
- New Umbral Blade spell types
- Soulfire capacity increases
- Grab strength upgrades (heavier objects)
- Parry window improvements

**Run-Specific:**
- Found weapons persist through rooms
- Environmental advantages/disadvantages
- Soulfire carries between encounters (risk/reward on spending)

---

## Design Pillars

### 1. Physics = Interactivity
If it exists in the world, it can be used as a weapon or tool.

### 2. Skill Over Stats
Mastery of Umbral Blade mechanics and combat flow > character level.

### 3. Aggressive Defense
Blocking ensures survival, attacking enables thriving.

### 4. Emergent Arsenal
Environment + dropped weapons + blade states = infinite tactical options.

### 5. Consistent Rules
- Projectiles collide mid-air
- Impaled objects are dashable
- Soulfire is universal currency
- Physics applies to everything

---

## Skill Ceiling Elements

### Beginner Level
- Basic impale → extract combos
- Simple dashing to blade
- Shield usage for safety
- Found weapon throwing

### Intermediate Level
- Directional extraction for positioning
- Dash-to-equip variations
- Managing Soulfire economy
- Environmental kills
- Blade positioning for zoning

### Expert Level
- Multi-target impale chains
- Mid-air blade redirects
- Bare-hand parrying
- Blade-anchoring while dual-wielding found weapons
- Perfect Soulfire management (never empty, never full)
- Environmental object juggling
- Predictive blade positioning

---

## Future Considerations

### Skill Slot System
- Limited number of active abilities (prevents button bloat)
- Passive modifications to core mechanics
- Encourages build diversity
- Allows for personal playstyle expression

### Advanced Mechanics (Post-MVP)
- Blade element types (fire/ice/lightning)
- Momentum-based damage scaling
- Sacrifice mechanics (blade takes hit for player)
- Chain reactions with environmental objects

---

## Technical Requirements

### Physics Engine Needs
- Precise collision detection for mid-air projectile blocking
- Ragdoll physics for enemy manipulation
- Destructible object fracturing
- Terrain deformation (visual + collision)
- Object weight/mass simulation for grab system

### Animation Systems
- Procedural blade movement
- Smooth state transitions
- Impalement penetration/extraction
- Player dash with variable endpoints
- Weapon degradation visual feedback

### AI Considerations
- Enemies must respect physics (can be grabbed, impaled, ragdolled)
- Need to react to blade positioning
- Should use environmental objects
- Varied sizes for grab tier system

---

*Document Version: 1.0*  
*Last Updated: [Current Date]*  
*Status: Early Design - Pre-Engine Implementation*