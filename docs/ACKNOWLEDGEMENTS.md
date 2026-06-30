# Acknowledgements

md2pdf is built on excellent open-source work. Huge thanks to the authors and
maintainers of the projects below.

## Primary dependencies

These are the crates md2pdf depends on directly.

| Crate | Version | License | Used for | Project |
| ----- | ------- | ------- | -------- | ------- |
| [pulldown-cmark](https://github.com/raphlinus/pulldown-cmark) | 0.9 | MIT | Parsing Markdown into events | raphlinus/pulldown-cmark |
| [genpdf](https://git.sr.ht/~ireas/genpdf-rs) | 0.2 | Apache-2.0 OR MIT | PDF layout & generation | ~ireas/genpdf-rs |
| [clap](https://github.com/clap-rs/clap) | 4 | MIT OR Apache-2.0 | Command-line argument parsing | clap-rs/clap |
| [eframe / egui](https://github.com/emilk/egui) | 0.27 | MIT OR Apache-2.0 | Cross-platform GUI | emilk/egui |
| [rfd](https://github.com/PolyMeilex/rfd) | 0.14 | MIT | Native file-open dialogs | PolyMeilex/rfd |
| [winresource](https://github.com/BenjaminRi/winresource) | 0.1 | MIT OR Apache-2.0 | Embedding the icon into the Windows `.exe` | BenjaminRi/winresource |

## Notable transitive dependencies

| Crate | License | Role |
| ----- | ------- | ---- |
| [printpdf](https://github.com/fschutt/printpdf) | MIT | Low-level PDF writer used by genpdf |
| [egui](https://github.com/emilk/egui) | MIT OR Apache-2.0 | Immediate-mode UI toolkit behind eframe |
| [image](https://github.com/image-rs/image) / [png](https://github.com/image-rs/image-png) | MIT OR Apache-2.0 | Image decoding for the GUI |
| [winit](https://github.com/rust-windowing/winit) | Apache-2.0 | Windowing for the GUI |

These crates pull in many smaller transitive dependencies (windowing, platform
bindings, text shaping, etc.). The dependency tree is predominantly licensed
under **MIT** and/or **Apache-2.0**.

To list the full resolved tree and every license yourself:

```sh
cargo tree                       # full dependency tree
cargo install cargo-license      # one-time
cargo license                    # license for every crate in the build
```

## Fonts

md2pdf embeds the **DejaVu** font family (DejaVu Sans and DejaVu Sans Mono) so
generated PDFs render consistently on any machine. The DejaVu fonts are
distributed under a permissive, free license (based on the Bitstream Vera and
Arev fonts licenses).

- Project: <https://dejavu-fonts.github.io/>
- Full license text: [`assets/fonts/DejaVu-LICENSE.txt`](../assets/fonts/DejaVu-LICENSE.txt)

## Tooling

- [Rust](https://www.rust-lang.org/) and [Cargo](https://doc.rust-lang.org/cargo/)
- [Inno Setup](https://jrsoftware.org/isinfo.php) — builds the Windows installer
- macOS `pkgbuild` / `productbuild` / `hdiutil` — build the `.pkg` and `.dmg`

## License

md2pdf itself is released under the [MIT License](../LICENSE). Each dependency
remains under its own license as listed above.
