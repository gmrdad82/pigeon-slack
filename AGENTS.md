# This repository is public

It is read by strangers, and its laws follow from that.

- **Nothing internal, ever.** No path under a home directory, no hostname
  but github.com, no personal tooling, no tunnel, no client, no person,
  no agent name, no internal item number. The gate in
  `.github/workflows/validate.yml` lists only generic patterns.
- **A release is the plugin.** A `v*` tag builds the plugin and attaches
  `plugin.toml`, `plugin.wasm` and `SHA256SUMS` to the GitHub release.
  A desk installs from this repository by name, reads the latest release,
  and verifies the artifact's hash against `SHA256SUMS` before loading it.
- **Hashes are read, never typed.** `SHA256SUMS` is computed by CI from
  the bytes it built.
- **The interface is upstream.** The `pito:host` world and the desk's own
  world come from the plugin template repository; they are copied here,
  never edited here.
- **Product names are the codenames** where the code needs one; the
  README speaks the shipped name.

# The local gate

```
cargo fmt --check
cargo check --target wasm32-wasip2
git diff --check
```

plus the two grep lines of `validate.yml`, run locally, both silent.

# Style

A short header comment per file saying what it owns. Plain prose, no
marketing. Security first.
