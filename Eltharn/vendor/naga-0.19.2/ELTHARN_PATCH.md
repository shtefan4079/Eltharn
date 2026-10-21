# Eltharn M03 raw-depth GLSL patch

Imported from the locally cached crates.io `naga 0.19.2` source, preserving
its MIT/Apache licenses. Upstream license texts (`LICENSE.MIT`,
`LICENSE.APACHE`) were copied from the sibling `vendor/wgpu-hal-0.19.5`
directory during open-source preparation, because the imported copy was
missing them; they are the standard MIT and Apache-2.0 texts. Pristine `src/back/glsl/mod.rs` SHA-256:
`8d4b1946f85869731bb9d7f197341bee04a94195c4a3f3df049576e4e452b10c`.
The original raw-depth patch changed only that Rust file. Cargo cache bookkeeping and the
upstream lockfile are omitted. This note is added for provenance.

Upstream rejects every WGSL `textureLoad` on a depth texture when emitting
GLSL. M03 needs the raw world depth on WebGL2. Modules containing a raw depth
load now emit ordinary `sampler2D`/`texelFetch(...).x`; the scalar zero branch
is preserved for `ReadZeroSkipWrite`. Function argument depth loads are also
detected using Naga's validated expression types.

The change deliberately rejects modules combining raw loads with comparison
samplers or filtered depth samples. Existing comparison-only modules retain
upstream `sampler2DShadow` output. M03's CPU admission exposes only raw depth
loads and dimensions. No GPU sample count, world depth format, camera data,
shaderpack bytes, default render pass or API version is changed by this patch.

A successful Rust compile does not prove GLSL linking or GPU depth semantics;
browser and device validation are not recorded here.

## Compiler maintenance (2026-10-20)

The pinned compiler exposed additional maintenance changes outside the original
GLSL patch. They retain the upstream licenses and do not change shader ABI:

- Remove compiler-confirmed redundant qualifications/parentheses and spell out
  already-inferred lifetimes, including the existing named lowering lifetime.
- Reserve the lowering locals map from the parser's local arena count. The
  arena's handle/span ownership remains intact; it is no longer an unread field.
- Replace deprecated IndexMap removal with `swap_remove`, its documented and
  inspected previous implementation; do not change expression ordering policy.
- Call the existing allocation-recycling trait through explicit UFCS where
  future standard-library Vec methods would otherwise collide.
- Run validator literal unit tests under `cfg(test)` instead of the nonexistent
  `validate` feature. The validator itself is unconditional in this release.

These repairs do not add blanket lint allowances or suppress compiler output.
Native library checks and synthetic backend cfg checks have bounded scope;
platform/device shader execution still requires the project's validation matrix.
The product source snapshot omits upstream tests and test-only helper code.
