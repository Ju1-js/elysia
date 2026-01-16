## Elysia Flatpak builds

Steps to build the Flatpak bundle:

```bash
# Generate cargo-sources
./flatpak-builder-tools/cargo/flatpak-cargo-generator.py ../Cargo.lock -o cargo-sources.json

# Build Flatpak
flatpak-builder --repo=repo --force-clean build-dir wine.dawn.dawnwinery.elysia.yml

# Generate elysia.flatpak bundle
flatpak build-bundle repo/ elysia.flatpak wine.dawn.dawnwinery.elysia
```
