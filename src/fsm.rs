/// Minimal finite-state-machine container.
///
/// `S` is the state type (usually an enum). The machine tracks the current
/// state, the previous state, and how long the machine has been in its current
/// state. **Transition logic is intentionally kept out of the machine itself**
/// — it lives in the ECS system (or an `impl S` method) that drives it.
///
/// # Usage
/// ```
/// let mut fsm = StateMachine::new(MyState::Idle);
/// // Each frame:
/// if let Some(next) = fsm.state.next(&ctx) { fsm.go(next); }
/// fsm.tick(dt);
/// ```
pub struct StateMachine<S: Clone> {
    pub state: S,
    pub previous: S,
    /// Seconds spent in the current state. Reset to 0.0 on each transition.
    pub elapsed: f32,
    entered_this_frame: bool,
}

impl<S: Clone> StateMachine<S> {
    /// Create a new machine starting in `initial`.
    /// `just_entered()` returns `true` on the first tick.
    pub fn new(initial: S) -> Self {
        Self {
            previous: initial.clone(),
            state: initial,
            elapsed: 0.0,
            entered_this_frame: true,
        }
    }

    /// Transition to `next` only if it is a **different variant** from the
    /// current state (compared by discriminant — no `PartialEq` required).
    /// Resets `elapsed` to 0.0 and sets `just_entered()` for one tick.
    pub fn go(&mut self, next: S) {
        if std::mem::discriminant(&self.state) != std::mem::discriminant(&next) {
            self.previous = std::mem::replace(&mut self.state, next);
            self.elapsed = 0.0;
            self.entered_this_frame = true;
        }
    }

    /// Like [`go`], but **always** transitions even if the variant is the same.
    /// Use when the variant carries data that changes (e.g. restarting a dash
    /// in a new direction without waiting for the old one to finish).
    pub fn force_go(&mut self, next: S) {
        self.previous = std::mem::replace(&mut self.state, next);
        self.elapsed = 0.0;
        self.entered_this_frame = true;
    }

    /// Advance the elapsed-in-state timer by `dt` seconds and clear the
    /// `just_entered` flag. Call once per frame **after** processing transitions.
    pub fn tick(&mut self, dt: f32) {
        self.elapsed += dt;
        self.entered_this_frame = false;
    }

    /// Returns `true` only on the first frame/tick after entering this state.
    pub fn just_entered(&self) -> bool {
        self.entered_this_frame
    }
}
