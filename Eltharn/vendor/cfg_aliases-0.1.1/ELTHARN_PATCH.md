# Compiler maintenance (2026-10-20)

Imported from the local crates.io cache for `cfg_aliases 0.1.1`, retaining
the upstream MIT license, `NOTICES.md`, README and manifest forms. Original
`src/lib.rs` SHA-256:
`83d97f5e7643893132c290cfa7754579535f6a1576ec0b4ecfea6e8759cfede9`.
Cargo cache bookkeeping is omitted.

Remove the three trailing semicolons from recursive expression-producing
macro arms. New Rust compilers diagnose those expansions with
`semicolon_in_expressions_from_macros`. This is the same narrow source
repair present in cached upstream `cfg_aliases 0.2.2`, applied to the 0.1
compatibility line used by WGPU 0.19 and Winit 0.29. No cfg parsing,
environment lookup, output flags or alias predicates change. No lint
allowance is added.

Validation compares emitted flags from pristine and patched macro sources
over synthetic target/feature cases. Patched probes compile with the
semicolon lint denied. These checks validate macro expansion and flags,
not platform compilation or execution.
