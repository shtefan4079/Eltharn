# Eltharn M03 raw-depth GLES defaults

Pristine crates.io `wgpu-hal 0.19.5` source imported from the local Cargo cache,
with its original licenses. `src/gles/device.rs` pristine SHA-256:
`2ed4180bf40deffff0f920eec846e2b2bac5bd2e66a4b9d4162cb1616ba093fb`.

Original M03 change: include `TextureSampleType::Depth` in the existing branch that
sets texture MIN/MAG filters to NEAREST on creation. Raw `texelFetch` has no
separate sampler object; GL's default LINEAR magnification can make float
depth textures incomplete. Explicit filtering/comparison sampler objects
continue to override these texture defaults. Vulkan, Metal and DX12 are unchanged.

The corresponding Naga GLSL patch enables raw depth loads. Both require browser
GPU validation; successful compilation alone is insufficient.

# Eltharn M08 WebGL attachment-limit GLES fix

Pristine `wgpu-hal 0.19.5` `src/gles/queue.rs` `C::ResetFramebuffer`
detached `COLOR_ATTACHMENT0..COLOR_ATTACHMENT7`
(`for i in 0..crate::MAX_COLOR_ATTACHMENTS`, `= 8`) on every non-default
FBO reset. An off-mode WebGL2 Chrome/SwiftShader baseline run emitted
`WebGL: INVALID_ENUM: framebufferTexture2D: invalid attachment` 256 times
(console cap); a direct capability query in the same Chrome flags returned
`MAX_COLOR_ATTACHMENTS=6`, so detaching indices 6 and 7 is the source-backed
suspected root cause. Hypothesis only: no live retest has confirmed the fix
yet; the original report is in Git at `c98ee1ba`.

Change (GLES only; Vulkan/Metal/DX12 untouched):
- `src/gles/mod.rs`: `Queue` gains `max_color_attachments: u8`.
- `src/gles/adapter.rs`: `open()` queries
  `gl.get_parameter_i32(glow::MAX_COLOR_ATTACHMENTS)` once, clamps via
  `clamp_max_color_attachments()` to `1..=crate::MAX_COLOR_ATTACHMENTS`,
  and stores it on the new `Queue` field. No per-frame `getParameter`.
- `src/gles/queue.rs`: `ResetFramebuffer` detaches
  `0..self.max_color_attachments` instead of `0..8`.
- Native GL drivers reporting >= 8 keep the previous detach behavior;
  WebGL2 drivers reporting fewer detach only their supported enums.

Validation still required: a live WebGL off/multipass comparison must show zero
`framebufferTexture2D: invalid attachment` lines; compilation alone is
insufficient.

## Compiler maintenance (2026-10-20)

The original backend predicates are retained in `build.rs`, now expressed
directly from Cargo's target/feature inputs. Every supported alias is declared
with `rustc-check-cfg`; enabled aliases still use `rustc-cfg`. This replaces the
old cfg_aliases expansion, which emitted expression-position semicolons and did
not declare custom cfg names. Its unused build dependency is removed from both
manifest forms. Native, Send/Sync, GLES, WebGL, Emscripten, Vulkan,
Metal and DX12 conditions remain separate and unchanged.

Compiler-confirmed redundant source qualifications and one unused import were
removed; the inferred EGL mutex-guard lifetime is explicit. No blanket warning
allowances, GPU format changes or new backend selection policy were introduced.
Synthetic target/feature cases verify cfg outputs for Linux, Windows, macOS,
Android, WebGL with/without atomics and Emscripten; these are build-predicate
checks, not execution on those platforms.

The actual `el_web` release check also compiles the WebGL branch for
`wasm32-unknown-unknown` with the application's atomics/build-std flags. It
passes without source warnings; the compiler still reports the unstable
`atomics` target feature. This is cross-compilation, not browser GPU execution.
