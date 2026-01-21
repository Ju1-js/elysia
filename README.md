<div align="center">

# elysia

*A launcher for anime games on Linux, supporting Wine and Proton* 💕

<img src="assets/images/ak-e.png" alt="Launcher" width="600"/>

> ⚠️ Elysia is still in very early development! Please report any issues. ⚠️

<img src="assets/elysia.png" alt="Elysia Launcher" width="200"/>

</div>

## Installation

### Flatpak

You can grab the latest `elysia.flatpak` from the [Releases](https://dawn.wine/elysia/elysia/releases) page and install it by double clicking it, or with `flatpak install elysia.flatpak`.

### AUR (coming soon!)

```
yay -S elysia-bin
```

### Native binary / Build from source

Note that you'll need `ffmpeg>=8.0` for Elysia to build correctly.

```
git clone https://dawn.wine/elysia/elysia.git
cd elysia
cargo build --release --bin elysia
```

---

## Support

Elysia is still very work-in-progress: if you meet any bugs, please make an issue or ask for help at [our discord server!](https://discord.gg/ck37X6UWBp)