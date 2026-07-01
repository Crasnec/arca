# Tauri for the archive manager

Arca's Graphical Interface will use Tauri: a WebView frontend packaged with Rust application commands that call `arca-core` directly. This accepts a web UI toolchain in exchange for a richer Archive Manager experience, while keeping archive operations inside the Rust process instead of shelling out to the CLI. The UI must present as a desktop Workbench, not as a website or landing page.

The Tauri command boundary returns GUI-specific DTOs instead of exposing `arca-core` structs directly. This keeps React-facing payloads in `camelCase`, preserves the CLI JSON shape, and lets the GUI manifest carry session state such as metadata validation, payload validation, password-required, and fully-validated flags without changing core command-line output.
