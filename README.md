# Terminal Miku 3D (Rust, CPU-Only)

Terminal renderer for 3D meshes and animations using ASCII/Braille-like glyph ramps.

## Features

- CPU-only software rasterization (no GPU, no WebGL).
- Perspective projection + triangle rasterization + z-buffer.
- Lambert shading mapped to terminal glyphs.
- OBJ loading for static meshes.
- GLB/glTF loading for static + skinned animated meshes.
- Interactive terminal playback with alternate screen + synchronized updates.

## Commands

```bash
cargo run -- run --scene cube --mode ascii --fps-cap 30
cargo run -- run --scene glb --glb /path/to/model.glb --anim 0 --mode ascii --fps-cap 30 --cell-aspect 0.5
cargo run -- run --scene obj --obj /path/to/model.obj --mode ascii --fps-cap 30
cargo run -- inspect --glb /path/to/model.glb
cargo run -- bench --scene cube --seconds 10
cargo run -- bench --scene obj --obj /path/to/model.obj --seconds 10
cargo run -- bench --scene glb-static --glb /path/to/model.glb --seconds 10
cargo run -- bench --scene glb-anim --glb /path/to/model.glb --anim 0 --seconds 10
```

More 3D-looking example (orbit + stronger highlights):

```bash
cargo run -- run --scene cube --fps-cap 30 --orbit-speed 0.8 --specular-strength 0.45 --rim-strength 0.35 --fog-strength 0.12
```

## Asset Policy

- Repository does not include model/motion assets.
- Keep local assets outside git tracking (`assets-local/`).
- For PMX/VMD workflows, convert to GLB offline first.
- See `/Users/user/miku/scripts/convert_mmd_to_glb.md`.

## Controls

- `q` or `Esc`: quit interactive mode

## Current Scope

- ASCII mode is primary target.
- Braille mode currently uses a brightness ramp, not full 2x4 subpixel rasterization.
# 3D-enjine
