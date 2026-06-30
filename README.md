<p align="center">
  <img src="assets/logo.png" alt="md2pdf logo" width="160">
</p>

<h1 align="center">md2pdf</h1>

<p align="center">A small, self-contained Markdown to PDF converter for macOS.</p>

A small, self-contained **Markdown -> PDF** converter for macOS, written in Rust.
It ships as both a simple desktop app and a command-line tool. Fonts are
embedded in the binary, so there are **no runtime dependencies** — no system
fonts, no headless browser, no LaTeX.

**Free and open source** (MIT licensed). Use it for anything you like, at no
cost. It is provided **as-is, with no warranty of any kind** — see
[Disclaimer](#disclaimer).

## Features

- Headings, **bold**, *italic*, `inline code`, and ~~strikethrough~~
- Fenced code blocks (monospace, framed) and inline code
- Ordered, unordered, and nested lists; task lists
- Blockquotes, horizontal rules, links (URL shown inline)
- Tables and image alt-text
- A4 / Letter / Legal paper sizes
- Automatic word-wrapping and page breaks

## The app (GUI)

Double-click **md2pdf.app**, then:

1. **Drag & drop** a Markdown file onto the window — or click **Browse…** to
   pick one.
2. **Output file name** — pre-filled with the source name and a `.pdf`
   extension; the PDF is written to the **same folder** as the source. Edit it
   if you like.
3. Pick a **paper size**, then **Convert to PDF**.

Use **Open PDF** / **Reveal in Finder** to jump straight to the result.

## The command-line tool

```sh
md2pdf input.md                 # → input.pdf (same folder)
md2pdf input.md -o report.pdf   # custom output
md2pdf README.md --paper letter --margin 18 --font-size 12
cat notes.md | md2pdf - -o notes.pdf   # read from stdin
```

Run `md2pdf --help` for all options.

## Installing

### Disk image (easiest)

Open **`dist/md2pdf-<version>-macos-<arch>.dmg`** and drag **md2pdf.app** onto
the **Applications** shortcut. The optional `cli/md2pdf` binary inside can be
copied to `/usr/local/bin` for command-line use.

### Installer package

Open **`dist/md2pdf-<version>.pkg`** and follow the prompts. It installs:

- `md2pdf.app` → `/Applications`
- `md2pdf` (CLI) → `/usr/local/bin`

### Portable archive

Unpack `dist/md2pdf-<version>-macos-<arch>.tar.gz` and move `md2pdf.app` to
`/Applications` and `md2pdf` somewhere on your `PATH`.

> **Gatekeeper note:** the binaries are signed ad-hoc, not notarized. The first
> launch may need right-click → **Open** (app), or
> `xattr -dr com.apple.quarantine md2pdf.app` to clear the quarantine flag.

## Building from source

Requires a recent Rust toolchain.

```sh
cargo build --release            # builds both md2pdf and md2pdf-gui
cargo run --bin md2pdf -- file.md
cargo run --bin md2pdf-gui       # launch the GUI

# Package everything for macOS (app + .pkg + tarball → dist/):
packaging/build-macos.sh
```

To produce a universal (Apple Silicon + Intel) build, install both targets
first:

```sh
rustup target add aarch64-apple-darwin x86_64-apple-darwin
```

## Project layout

| Path | Purpose |
| ---- | ------- |
| `src/lib.rs` | library root |
| `src/render.rs` | Markdown → PDF engine (pulldown-cmark → genpdf) |
| `src/bin/md2pdf.rs` | command-line tool |
| `src/bin/md2pdf-gui.rs` | egui desktop app |
| `assets/fonts/` | embedded DejaVu fonts |
| `packaging/` | macOS app/installer build scripts |

## Disclaimer

md2pdf is free, open-source software provided **"as is", without warranty of
any kind**, express or implied, including but not limited to the warranties of
merchantability, fitness for a particular purpose, and noninfringement. You use
it entirely at your own risk; the authors are not liable for any claim, damages,
or other liability arising from its use. See the [LICENSE](LICENSE) for the full
terms.

## License

MIT — see [LICENSE](LICENSE). Bundles the DejaVu fonts (see
`assets/fonts/DejaVu-LICENSE.txt`).
