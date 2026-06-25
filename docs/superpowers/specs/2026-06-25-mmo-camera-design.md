# mmo_camera — Design Spec (Future Crate)

**Date:** 2026-06-25  
**Crate:** `crates/mmo_camera` (not yet created)  
**Status:** Planned — implement after `mmo_input` is complete  
**Depends on:** `mmo_input`

---

## Goal

A Bevy plugin crate that owns a follow camera for MMO-style 3D games. Reads `MmoMovementState` and `MmoBindings` from `mmo_input`, manages a camera entity that orbits the player, and writes `MmoCameraOrientation` back to the player entity so the `mmo_input` tnua feature can rotate movement vectors correctly.

---

## Dependencies

```toml
[dependencies]
bevy = { version = "0.19", default-features = false }
mmo_input = { path = "../mmo_input" }
serde = { version = "1", features = ["derive"] }
```

No physics dependency — the camera is purely transform-driven.

---

## Plugin API

```rust
app.add_plugins(MmoCameraPlugin::default());
```

The plugin:
- Spawns a camera entity as a child of the player (or as a free entity with a configurable follow target)
- Registers systems that read `MmoMovementState` and `MmoBindings`
- Writes `MmoCameraOrientation` to the player entity each frame

---

## Player Entity Setup

```rust
commands.spawn((
    // from mmo_input:
    MmoMovementContext,
    MmoMovementState::default(),
    MmoMovementParams::default(),
    MmoMovementMask::ALL,
    // from mmo_camera:
    MmoCameraTarget,               // marker — camera plugin follows this entity
    MmoCameraOrientation::default(), // written by plugin, read by mmo_input tnua feature
    MmoCameraParams::default(),    // behavior tuning (not rebindable)
));
```

---

## Public Types

### `MmoCameraTarget`

Marker component. The plugin follows exactly one entity with this component (for a single local player). Multi-player split-screen would require rework.

### `MmoCameraOrientation`

Written by `mmo_camera`, read by `mmo_input`'s tnua feature to rotate `move_intent` into world space.

```rust
#[derive(Component, Reflect, Default, Clone, Copy)]
pub struct MmoCameraOrientation {
    pub yaw: f32,       // radians; 0 = world +Z forward
    pub pitch: f32,     // radians; clamped by MmoCameraParams
    pub distance: f32,  // current zoom distance in world units
}
```

### `MmoCameraParams`

Per-entity behavior tuning. Not user-rebindable (those live in `MmoBindings`).

```rust
#[derive(Component, Reflect, Clone)]
pub struct MmoCameraParams {
    pub min_distance: f32,     // closest zoom; default: 1.0
    pub max_distance: f32,     // farthest zoom; default: 30.0
    pub default_distance: f32, // starting zoom; default: 8.0
    pub pitch_min: f32,        // radians; default: -0.4 (slightly below horizon)
    pub pitch_max: f32,        // radians; default: 1.4 (nearly top-down)
    pub yaw_smooth: f32,       // lerp factor for yaw; default: 1.0 (instant)
    pub pitch_smooth: f32,     // lerp factor for pitch; default: 1.0 (instant)
    pub collision_offset: f32, // pull camera in when occluded; default: 0.3
}
```

---

## Camera Modes

### Mode 1: Locked (no button held, keyboard/mouse)

- Camera stays fixed at current yaw/pitch behind the player.
- Character turn (A/D in non-mouse-look mode) rotates both the character and the camera yaw together, keeping camera behind.
- `is_mouse_look == false` → camera follows character yaw passively.

### Mode 2: RMB action-cam (RMB held, keyboard/mouse)

- `MmoMovementState.camera_delta.x` drives camera yaw (and character yaw simultaneously).
- `MmoMovementState.camera_delta.y` drives camera pitch.
- `is_mouse_look == true` → the plugin writes `MmoCameraOrientation.yaw` from accumulated delta; `mmo_input` tnua feature reads this yaw to orient walk direction.

### Mode 3: Gamepad (always action-cam equivalent)

- `MmoMovementState.is_mouse_look` is always `true` when `input_source == Gamepad`.
- Right stick X/Y → `camera_delta` → orbit yaw/pitch.
- Character always moves camera-relative (FFXIV style).

---

## Camera Entity Structure

```
Player entity (MmoCameraTarget)
└── CameraArm entity (Transform offset by distance along -Z in camera space)
    └── Camera3d entity
```

The arm entity's transform is updated each frame from `MmoCameraOrientation`. No spring arm physics in v1 — distance is set directly (with optional collision pull-in).

---

## Systems

### `update_orientation`

Runs every frame in `Update` after `mmo_input`'s `update_state`.

1. Reads `MmoMovementState.camera_delta` and `is_mouse_look`.
2. If `is_mouse_look`: accumulates `camera_delta.x` into `MmoCameraOrientation.yaw`, `camera_delta.y` into `pitch` (clamped by `MmoCameraParams`).
3. If not `is_mouse_look` (locked mode): syncs `orientation.yaw` to the player entity's current `Transform` yaw so the camera stays behind.
4. Applies smoothing factors from `MmoCameraParams`.

### `apply_zoom`

Reads `MmoBindings.zoom_in / zoom_out` and scroll input. Adjusts `orientation.distance` within `[params.min_distance, params.max_distance]` scaled by `MmoBindings.zoom_speed`.

### `update_camera_transform`

Runs in `PostUpdate` after transforms are propagated. Positions the camera arm entity based on current `MmoCameraOrientation` (yaw, pitch, distance). Applies collision pull-in if occluded.

---

## Interface with `mmo_input`

Reads from `mmo_input`:
- `MmoMovementState.camera_delta` — per-frame orbit intent
- `MmoMovementState.is_mouse_look` — mode selection
- `MmoMovementState.input_source` — gamepad vs kb/mouse mode
- `MmoBindings.zoom_in / zoom_out / zoom_speed` — zoom input handling
- `MmoBindings.mouse_sensitivity / gamepad_sensitivity` — already applied to `camera_delta`; no double-scaling needed

Writes to player entity:
- `MmoCameraOrientation` — consumed by `mmo_input`'s tnua feature

---

## Out of Scope (v1)

- LMB free-look (camera orbits freely, snaps back on release) — design later
- Camera collision (full raycast spring arm) — `collision_offset` is a simple pull-in only
- Multiple simultaneous camera targets (split-screen)
- Cinematic / cutscene camera overrides
- First-person mode (distance = 0 special case)
