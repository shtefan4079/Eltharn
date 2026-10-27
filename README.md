# Eltharn

A Rust Minecraft-compatible client with the Eltharn Gate launcher.
**Pre-alpha: the project is unfinished.**

Third-party code
retains its original licenses.

## Build

Use Rustup and the pinned toolchain. On Linux, native development libraries for
ALSA, Wayland, XKB and udev are required.

```sh
cargo build --release --locked -p eltharn
cargo run --release --locked -p eltharn
```

The desktop build produces one executable with the launcher, game and 1.21.11
IPC backend. Minecraft assets are downloaded when needed.

[License](LICENSE): PolyForm Noncommercial 1.0.0.
Commercial licensing and contact: [Eltharn145@gmail.com](mailto:Eltharn145@gmail.com).

NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG
OR MICROSOFT.
