# Surunizer

**Surunizer** is a specialized image customizer, palette recoloring tool, and viewer built in Rust with [`egui`](https://github.com/emilk/egui) and [`eframe`](https://github.com/emilk/egui/tree/master/crates/eframe), crafted specifically for tailoring icons and graphics to the **Suru++** design ecosystem.

![Surunizer Icon](surunizer_icon.png)

## Features

- **Suru++ Palette Transformation**:
  - **Style 1 (Suru BlueGrey)**: Maps image luminance across the iconic Suru++ palette (`#455a64`, `#78909c`, `#fcefe8`, `#ffffff`).
  - **Style 3 (Inverted)**: Inverted high-contrast Suru++ luminance mapping.
  - **Original**: View and adjust without palette remapping.
- **Multi-Format Support**:
  - Raster: PNG, JPEG, WebP, BMP, ICO.
  - Vector: Opens SVG files (rendered crisply via `rsvg-convert` or ImageMagick).
  - SVG Export: Save customized output directly to SVG or raster formats (PNG, JPEG, WebP).
- **Interactive Color Keying (Transparency)**:
  - Click-to-pick background color keying with real-time Euclidean distance tolerance.
  - Easily remove solid backgrounds from existing icons and graphics.
- **Cropping & Adjustments**:
  - Coordinate-precise image cropping (X, Y, Width, Height).
  - Fine-grained brightness and contrast sliders with 1-click reset.
- **Zoom & Checkerboard Canvas**:
  - Smooth zoom scaling (0.1× to 5.0×) centered over an alpha checkerboard grid.
- **Desktop & CLI Integration**:
  - Open images via command line: `surunizer [path-to-image]`.
  - Drag and drop images directly into the application window.
  - Full Linux desktop integration (`.desktop` entry and MIME associations).
- **Persistent Preferences**:
  - Automatically remembers last accessed directories, style mode, zoom, and adjustments in `~/.config/surunizer/config.json`.

## Building & Installation

### Prerequisites

Ensure you have Rust and Cargo installed via [rustup](https://rustup.rs/):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

For SVG rendering support, ensure `librsvg` is installed on your system:
```bash
# Arch Linux
sudo pacman -S librsvg

# Debian / Ubuntu
sudo apt install librsvg2-bin

# Fedora
sudo dnf install librsvg2-tools
```

### Build from Source

```bash
# Clone the repository
git clone https://github.com/zmata535-dev/surunizer.git
cd surunizer

# Build optimized release binary
cargo build --release

# Run
./target/release/surunizer [path-to-image]
```

### Desktop Integration (Linux)

To install Surunizer to your desktop applications menu:

```bash
# Copy binary
cp target/release/surunizer ~/.local/bin/

# Copy desktop entry and application icon
cp surunizer.desktop ~/.local/share/applications/
cp surunizer_icon.png ~/.local/share/pixmaps/surunizer.png

# Update desktop database
update-desktop-database ~/.local/share/applications/
```

## Usage

1. **Open an Image**: Click **Open...**, drag-and-drop a file, or launch with `surunizer /path/to/icon.svg`.
2. **Select Style**: Choose **Style 1 (Suru)** or **Style 3 (Inverted)** in the sidebar to map colors to the Suru++ palette.
3. **Key Out Background**: Click **🎯 Pick Color for Transparent**, then click on any background pixel in the viewer to key it out as transparent. Adjust the **Tolerance** slider as needed.
4. **Crop & Tweak**: Adjust Brightness, Contrast, or enable **Crop Image** to focus on the icon glyph.
5. **Save**: Click **Save** to overwrite the file or **Save As...** to export to SVG, PNG, JPEG, or WebP.

## License

Dual-licensed under either:

- MIT License ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)

at your option.
