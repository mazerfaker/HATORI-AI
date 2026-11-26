# Windows installer

CI builds the Rust executable and then compiles HATORI.iss using Inno Setup 6. The installer is per-user and does not require administrator rights. Local memory in %LOCALAPPDATA%/HATORI_AI is not removed by uninstall. Optional startup and desktop shortcuts are provided. The runtime directory is optional; no llama.cpp binaries or GGUF weights are bundled yet. The installer is unsigned. Close HATORI before updating. Existing manually configured runtime paths may need updating after moving the executable.
