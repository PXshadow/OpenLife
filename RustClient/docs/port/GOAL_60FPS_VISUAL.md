# Client visual/perf goal (iterate toward Jason parity)

## Target
- **60 FPS** soft-FB + GPU present on a typical desktop (960×540 buffer).
- **Fullscreen** fills the monitor (no letterboxed postage stamp).
- **Windowed** ~960×540 (not 2× zoomed loading / not 2560×1440).
- **Animations** (objects + players): dual-fade packs, walk, held, ground object anims — same hooks as Haxe/Jason (`anim_bank` / `step_anims_with_sounds` / `ObjectAnimPack`).
- **Sounds**: scene + session banks volume applied; footstep / use / emot / map ground anim via OLSN + cpal when `--features audio`.
- **Look**: soft/square ground like LivingLifePage; density via zoom ~48–96.

## References
- Jason: `LivingLifePage.cpp` draw/ground/anim, `animationBank.cpp`, `soundBank.cpp`, `groundSprites.cpp`
- Haxe: `openlife/client/Render.hx`, `Object.hx`, `Sound.hx`
- Port kit: `RustClient/docs/port/ARCHITECTURE_*.md`, `CALL_INDEX.md`

## Done this pass
- Fullscreen setting (`fullscreen=0/1`, F3 → Fullscreen)
- Soft windows Scale X1; loading/account not zoomed
- GPU buffer 960×540; borderless fullscreen fills surface
- Revert bilinear (was ~1 FPS); nearest + solid underfill + soft-edge TGAs
- Zoom default 48; step_move_pos for smooth walk camera
- Apply SFX volume to scene.sounds

## Next if still short of 60
- Cap soft-ground 2× draws; more square tiles
- Dirty/static ground cache
- True GPU sprite batch (wgpu quads) beyond present-only
