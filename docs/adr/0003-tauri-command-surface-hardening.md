# Harden the Tauri command surface

Arca's Graphical Interface will expose archive operations through narrow Tauri commands rather than broad filesystem access from the renderer. This keeps the WebView UI from becoming the trust boundary: paths, archive policy, permissions, and operation lifetimes remain enforced by Rust-side commands backed by `arca-core`.
