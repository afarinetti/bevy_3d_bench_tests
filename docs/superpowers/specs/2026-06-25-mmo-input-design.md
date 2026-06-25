# mmo_input — Design Spec

**Date:** 2026-06-25  
**Crate:** `crates/mmo_input`  
**Status:** Approved, pending implementation plan

---

## Goal

A reusable Bevy plugin crate that implements MMO-style movement controls (WoW keyboard/mouse, FFXIV gamepad) using `bevy_enhanced_input`, outputting a `MmoMovementState` component each frame. Optionally drives a `TnuaController` walk basis via a cargo feature. Designed to be consumed by any Bevy 3D game using `avian3d` + `bevy_tnua`.

---

## Dependencies

```toml
[dependencies]
bevy = { version = "0.19", default-features = false }
bevy_enhanced_input = "0.26"
bitflags = "2"
serde = { version = "1", features = ["derive"] }

[dependencies.bevy_tnua]
version = "*"
optional = true

[features]
default = []
tnua = ["dep:bevy_tnua"]
```

---

## File Layout

```
crates/mmo_input/src/
├── lib.rs          # MmoMovementPlugin, builder, re-exports
├── actions.rs      # InputAction definitions
├── bindings.rs     # MmoBindings resource, UserBinding wrapper, defaults
├── state.rs        # MmoMovementState, MmoMovementMask, InputSource
├── params.rs       # MmoMovementParams
├── systems/
│   ├── mod.rs
│   ├── bindings.rs # reactive rebuild when MmoBindings changes
│   └── state.rs    # action state → MmoMovementState each frame
└── tnua.rs         # feature-gated: MmoMovementState → TnuaController basis
```

---

## Plugin API

```rust
app.add_plugins(
    MmoMovementPlugin::builder()
        .bindings(MmoBindings::default())  // optional — override defaults
        .enable_tnua()                     // optional — requires tnua feature
        .build()
);
```

The builder inserts `MmoBindings` as a `Resource` and registers `MmoMovementContext` as a `bevy_enhanced_input` input context.

---

## Player Entity Setup

```rust
commands.spawn((
    MmoMovementContext,            // marker — plugin wires up bevy_enhanced_input actions
    MmoMovementState::default(),   // written every frame by the plugin
    MmoMovementParams::default(),  // read by tnua feature; write from buff/debuff systems
    MmoMovementMask::ALL,          // bitflags — status effects and UI write to this
    // game-owned:
    Transform::default(),
    RigidBody::Dynamic,
    TnuaController::default(),     // if using tnua feature
));
```

---

## Public Types

### `MmoMovementState`

Written every frame. Values are zeroed for axes suppressed by `MmoMovementMask`.

```rust
#[derive(Component, Reflect, Serialize, Deserialize, Default, Clone)]
pub struct MmoMovementState {
    /// Camera-relative movement intent. X = strafe (right+), Y = forward (+).
    /// Keyboard: 0.0 or ±1.0. Gamepad left stick: analog ±1.0.
    pub move_intent: Vec2,

    /// Vertical movement intent for swimming/flying. +1 = up, -1 = down.
    pub vertical_intent: f32,

    /// Character yaw change this frame in radians.
    /// Source: A/D turn (no RMB), mouse X (RMB held), gamepad right stick X.
    pub yaw_delta: f32,

    /// Raw camera orbit delta in radians. X = yaw, Y = pitch.
    /// Consumed by mmo_camera; scaled by mouse_sensitivity / gamepad_sensitivity.
    pub camera_delta: Vec2,

    /// Jump binding was pressed this frame. Signal only — game decides height,
    /// allow_in_air, cooldowns, etc.
    pub jump: bool,

    /// Sprint binding was pressed this frame. Signal only — game decides whether
    /// to trigger a sprint ability, temporarily raise speed_multiplier, etc.
    /// Maps to L3 (gamepad) or a configurable key (keyboard).
    pub sprint: bool,

    /// Auto-run is toggled on.
    pub auto_run: bool,

    /// RMB is held (kb/mouse) or any gamepad input is active (gamepad is always
    /// camera-relative, so this is always true when input_source == Gamepad).
    pub is_mouse_look: bool,

    /// Which input device drove this frame's state.
    pub input_source: InputSource,

    /// Monotonic tick counter. Incremented each frame the plugin runs.
    /// Attach to network packets for client-side prediction / reconciliation.
    pub tick: u64,
}

#[derive(Reflect, Serialize, Deserialize, Clone, Copy, Default, PartialEq, Eq)]
pub enum InputSource {
    #[default]
    KeyboardMouse,
    Gamepad,
}
```

### `MmoMovementMask`

Controls which axes the state system writes. Status effects, UI, cutscenes all write here.

```rust
bitflags::bitflags! {
    #[derive(Component, Reflect, Serialize, Deserialize, Clone, Copy)]
    pub struct MmoMovementMask: u8 {
        const MOVEMENT = 0b0001;  // move_intent
        const TURNING  = 0b0010;  // yaw_delta only — camera_delta is never masked
        const JUMPING  = 0b0100;  // jump + sprint signals
        const VERTICAL = 0b1000;  // vertical_intent
        const ALL      = 0b1111;
    }
}
```

`TURNING` zeroes `yaw_delta` (character cannot rotate) but never zeroes `camera_delta`. A rooted or stunned character can always orbit the camera freely.

Usage:
```rust
// Stun (no movement or jumping, can still turn camera):
mask.remove(MmoMovementMask::MOVEMENT | MmoMovementMask::JUMPING | MmoMovementMask::VERTICAL);

// Root (can't move position, can still turn):
mask.remove(MmoMovementMask::MOVEMENT | MmoMovementMask::VERTICAL);

// UI / chat box open (suppress everything):
*mask = MmoMovementMask::empty();

// Restore:
*mask = MmoMovementMask::ALL;
```

### `MmoMovementParams`

Tweakable walk configuration. The `tnua` feature reads this every frame to build `TnuaBuiltinWalk`. Buff/debuff/mount systems write to it.

```rust
#[derive(Component, Reflect, Clone)]
pub struct MmoMovementParams {
    pub walk_speed: f32,         // base speed in units/sec; default: 7.0
    pub speed_multiplier: f32,   // 1.0 = normal; set by spells/mounts/snares; default: 1.0
    pub float_height: f32,       // tnua hover height; default: 0.5
    pub acceleration: f32,       // default: 60.0
    pub air_acceleration: f32,   // default: 10.0
    pub coyote_time: f32,        // seconds; default: 0.15
}
```

### `MmoBindings`

Serializable. Insert as a `Resource` via the builder. Mutate at runtime for in-game remapping menus; the plugin watches for changes and rebuilds binding entities automatically.

```rust
#[derive(Resource, Reflect, Serialize, Deserialize, Clone)]
pub struct MmoBindings {
    // -- Keyboard / mouse (WoW-style defaults) --
    pub move_forward:  Vec<UserBinding>,   // [W, Up]
    pub move_backward: Vec<UserBinding>,   // [S, Down]
    pub turn_left:     Vec<UserBinding>,   // [A, Left]
    pub turn_right:    Vec<UserBinding>,   // [D, Right]
    pub strafe_left:   Vec<UserBinding>,   // [Q]
    pub strafe_right:  Vec<UserBinding>,   // [E]
    pub jump:          Vec<UserBinding>,   // [Space]
    pub mouse_look:    Vec<UserBinding>,   // [MouseButton::Right]
    pub left_mouse:    Vec<UserBinding>,   // [MouseButton::Left]
    pub auto_run:      Vec<UserBinding>,   // [NumLock]

    // -- Sprint (signal only; no default kb binding for WoW-style) --
    pub sprint: Vec<UserBinding>,          // [] by default; game can assign e.g. Shift

    // -- Gamepad (FFXIV-style defaults) --
    // Left stick (axes) and right stick (axes) are hardcoded — not rebindable.
    pub gamepad_jump:   Vec<UserBinding>,  // [South / Cross]
    pub gamepad_sprint: Vec<UserBinding>,  // [LeftThumb / L3]

    // -- Camera (consumed by mmo_camera) --
    pub zoom_in:   Vec<UserBinding>,  // [ScrollUp]
    pub zoom_out:  Vec<UserBinding>,  // [ScrollDown]
    pub zoom_speed: f32,              // default: 2.0

    // -- Sensitivity --
    pub mouse_sensitivity:   f32,  // scales raw mouse delta → camera_delta; default: 0.003
    pub gamepad_sensitivity: f32,  // scales right stick → camera_delta; default: 2.5
    pub kb_turn_speed:       f32,  // radians/sec for A/D keyboard turn; default: π
}
```

`UserBinding` is a thin serde-friendly newtype over `bevy_enhanced_input`'s `Binding` enum, providing `Serialize`/`Deserialize` via string representation (e.g. `"KeyCode::W"`, `"GamepadButton::South"`).

---

## Actions (internal)

Defined in `actions.rs`, not part of the public API. Each is a zero-size struct deriving `InputAction`.

| Action | Output | Default binding(s) |
|---|---|---|
| `MoveForward` | `bool` | W, Up |
| `MoveBackward` | `bool` | S, Down |
| `TurnLeft` | `bool` | A, Left |
| `TurnRight` | `bool` | D, Right |
| `StrafeLeft` | `bool` | Q |
| `StrafeRight` | `bool` | E |
| `Jump` | `bool` | Space, Gamepad South |
| `MouseLook` | `bool` | RMB |
| `LeftMouse` | `bool` | LMB |
| `AutoRun` | `bool` | NumLock |
| `MouseDelta` | `Vec2` | MouseMotion |
| `GamepadMove` | `Vec2` | Left stick (hardcoded) |
| `GamepadCamera` | `Vec2` | Right stick (hardcoded) |
| `Sprint` | `bool` | L3 (gamepad), none by default (kb/mouse) |

All actions use `consume_input: false` so they never block ability or UI systems from seeing the same key events.

---

## Systems

### `systems::bindings::rebuild_bindings`

Runs when `MmoBindings` is changed (via `Changed<Res<MmoBindings>>`). Despawns the current `bevy_enhanced_input` action child entities on every `MmoMovementContext` entity and respawns them using the updated `MmoBindings`. This is the mechanism behind runtime remapping.

### `systems::state::update_state`

Runs every frame in `Update`. For each entity with `(MmoMovementContext, MmoMovementState, MmoMovementMask)`:

1. Reads `Action<T>` components (pull-style) for all movement actions.
2. Determines `input_source` from whether gamepad axes are non-zero.
3. Applies WoW dual-mode logic:
   - If `MouseLook` is held **or** `input_source == Gamepad`: A/D → strafe (added to `move_intent.x`), not turn.
   - If keyboard-only (no `MouseLook`): A/D → `yaw_delta` at `kb_turn_speed`.
4. Handles both-mouse-buttons-forward: if `LeftMouse` and `MouseLook` are both held, adds `1.0` to `move_intent.y`.
5. Applies auto-run toggle on `AutoRun` action fired event (not held).
6. If `auto_run`, ensures `move_intent.y >= 1.0`.
7. Applies `MmoMovementMask`: zeros suppressed axes before writing.
8. Increments `tick`.

### `tnua::drive_walk_basis` (feature = "tnua")

Runs every frame after `update_state`. For each entity with `(MmoMovementState, MmoMovementParams, TnuaController)`:

1. Reads `MmoCameraOrientation` from the entity if present (written by `mmo_camera`). Falls back to the entity's own `Transform::forward()` if absent.
2. Rotates `state.move_intent` (camera-relative `Vec2`) into world-space `Vec3` using camera yaw.
3. Applies `params.speed_multiplier`.
4. Manually constructs and writes `TnuaBuiltinWalk { desired_motion, desired_forward, float_height, acceleration, air_acceleration, coyote_time, .. Default::default() }` from `MmoMovementParams` fields. No `From` impl exists in bevy_tnua — this is an explicit field mapping.
5. Does **not** feed any tnua actions (jump, dash, crouch). Those remain the game's responsibility.

---

## Keyboard/Mouse Behaviour Summary (WoW-style)

| Condition | A / D | Mouse X |
|---|---|---|
| No button held | Turn character | No effect |
| RMB held | Strafe | Turn character + camera |
| LMB + RMB held | Strafe | Turn character + camera; auto-move forward |
| Gamepad | Always strafe (left stick) | Right stick always orbits camera + turns character |

---

## Interface with `mmo_camera`

`mmo_input` writes; `mmo_camera` reads:
- `MmoMovementState.camera_delta` — scaled orbit intent (yaw/pitch radians)
- `MmoMovementState.is_mouse_look` — whether camera yaw should couple to character yaw
- `MmoBindings.zoom_in / zoom_out / zoom_speed` — camera zoom bindings

`mmo_camera` writes; `mmo_input` tnua feature reads:
- `MmoCameraOrientation { yaw: f32, pitch: f32, distance: f32 }` — component on the player entity

`mmo_camera` is an optional dependency. Everything in `mmo_input` degrades gracefully if it is absent.

---

## Out of Scope (v1)

- Click-to-move
- LMB free-look camera (planned for `mmo_camera`)
- Ability hotbar input
- Vehicle / swimming physics (vertical_intent field is present; physics backend hookup is game code)
- Networked input replication (tick field enables this; serialization/transport is game code)
