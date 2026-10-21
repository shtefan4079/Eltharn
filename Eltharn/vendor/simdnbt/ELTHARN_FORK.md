# Eltharn fork of `simdnbt`

This directory is a vendored fork of [simdnbt](https://github.com/azalea-rs/simdnbt)
`0.10.0` (MIT, upstream `LICENSE` retained; the vendored `simdnbt-derive`
subcrate is kept alongside it). It is consumed by the 1.21.11 protocol backend.

The delta below was introduced when the Android build panicked inside
`NbtTag`/`NbtList` conversions on ARM64. **The upstream root cause is
unverified**: the changes were a targeted workaround, not a diagnosed bug
report, and no minimal reproducer or upstream issue exists in this checkout.
Treat the fork as a temporary carry.

## Delta vs upstream 0.10.0

### 1. `read_volatile` on the raw tape-element reads

- `src/borrow/list.rs` — `NbtList::element()`
- `src/borrow/mod.rs` — `NbtTag::element()`
- `src/borrow/compound.rs` — `NbtCompound::element()`

Upstream reads the tape element with `unsafe { *self.element }`. The fork uses
`unsafe { self.element.read_volatile() }` and comments the rationale: without
it the optimizer may reuse/reorder a cached raw-pointer dereference across
call sites, so two reads of the same address can disagree. On ARM64 this is
the suspected cause of the panics; see the warning below.

### 2. Silent fallback in `NbtList::to_owned`

`src/borrow/list.rs`: upstream panics with
`unreachable!("this is an NbtList, no other kinds should be possible")` for an
unexpected tape kind; the fork returns `crate::owned::NbtList::Empty`
instead.

### 3. Silent fallback in `PartialEq for NbtList`

`src/borrow/list.rs`: two lists whose tape kind is not one of the known list
kinds now compare equal (`_ => true`) instead of hitting `unreachable!`.
This keeps `Debug`/equality checks from panicking in the same situation as
(2). It deliberately does not make the comparison meaningful.

### 4. `NbtTag::to_owned` avoids a second element read

`src/borrow/mod.rs`: the list branch of `NbtTag::to_owned` used to call
`self.list()`, which dereferences the element pointer a second time (and
`unwrap()`s the `Option`); it now goes through a private
`list_with_element(_el)` that reuses the already-read `TapeElement` and can
therefore never disagree with the kind check in the same call.

### 5. Vendoring/build deltas (no decoder behavior)

`Cargo.toml` retains only the runtime library and derive crate. Upstream tests,
examples, benchmarks, their development dependencies, and local lockfiles are
omitted from this product snapshot. `.cargo_vcs_info.json` records the source
revision; `LICENSE` is retained.

## Warning

Items 2 and 3 replace panics with silent, wrong answers (`Empty` / `true`).
They hide exactly the class of bug the `read_volatile` fix is meant to
prevent, and they silently mis-deserialize data if reached. The unsafe code is
intentionally left as upstream wrote it apart from the deltas above; do not
extend the silent fallbacks. If upstream ships a root-cause fix, re-vendor
instead of layering more workarounds here.
