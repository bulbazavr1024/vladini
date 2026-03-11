# Image Preparer CLI

Command-line tool for compressing images/videos, converting between formats, and stripping metadata.

> **Note**: This is part of the Image Preparer workspace. For the HTTP API server, see `../server/README.md`. For workspace documentation, see `../README.md`.

## Features

- ✅ **PNG** - Lossy/Lossless compression (50-90% reduction)
- ✅ **WebP** - Lossy/Lossless compression (40-80% reduction)
- ✅ **JPEG** - Compression + format conversion
- ✅ **MP3** - Metadata stripping (ID3 tags)
- ✅ **MP4** - Video compression (70-96% reduction) + Frame extraction
- 🔄 **Format conversion** - PNG ↔ JPG ↔ WebP
- 🚀 **Parallel processing** for batch operations
- 📊 **Metadata inspection** without modification
- 🎯 **Configurable quality/speed trade-offs**
- 📐 **Image resizing** with aspect ratio preservation

## Installation

### Quick Install

From the workspace root:
```bash
cargo install --path cli
```

Or from this directory:
```bash
cd cli
cargo install --path .
```

This installs `image_preparer` to `~/.cargo/bin/` (already in PATH).

### Prerequisites

- Rust 1.70+ (install from [rustup.rs](https://rustup.rs))
- **ffmpeg** (required for MP4 processing)
  ```bash
  # macOS
  brew install ffmpeg

  # Linux
  apt install ffmpeg
  ```

See [INSTALL.md](./INSTALL.md) for more installation options.

## Commands

The tool uses subcommands for different operations:

- `compress` - Compress images or videos
- `convert` - Convert between image formats
- `inspect` - Display file metadata
- `extract` - Extract frames from videos

## Usage

### Compress Command

Compress images and videos with configurable quality.

```bash
# Compress single file
image_preparer compress photo.png -q 80

# Compress with high quality
image_preparer compress photo.png -q 90

# Fast compression (lower quality)
image_preparer compress photo.png -q 50 -s 10

# Lossless optimization only
image_preparer compress photo.png --no-lossy

# Process entire directory
image_preparer compress ./photos -r

# Strip all metadata
image_preparer compress photo.png --strip all

# Keep safe metadata
image_preparer compress song.mp3 --strip safe

# Compress video
image_preparer compress video.mp4 -q 70

# With output path
image_preparer compress input.png output.png

# Resize to 800px width (height auto-calculated)
image_preparer compress photo.png --width 800

# Resize to exact 1920x1080
image_preparer compress photo.png --width 1920 --height 1080

# Resize height only (width auto-calculated)
image_preparer compress photo.png --height 600

# Resize + compress entire directory
image_preparer compress ./photos -r -q 85 --width 1200
```

**Options:**
- `-q, --quality <0-100>` - Quality level (default: 80)
- `-s, --speed <1-10>` - Speed vs quality (default: 3)
- `--no-lossy` - Lossless mode only
- `--strip <all|safe|none>` - Metadata stripping (default: all)
- `-r, --recursive` - Process directories
- `--backup` - Create .bak backups
- `--dry-run` - Preview changes
- `--width <pixels>` - Resize width (aspect ratio preserved if height omitted)
- `--height <pixels>` - Resize height (aspect ratio preserved if width omitted)

### Convert Command

Convert images between PNG, JPG, and WebP formats.

```bash
# Convert PNG to JPG
image_preparer convert photo.png photo.jpg --to jpg

# Convert with specific quality
image_preparer convert image.png image.jpg --to jpg -q 90

# Convert PNG to WebP (lossy)
image_preparer convert photo.png photo.webp --to webp -q 80

# Convert PNG to WebP (lossless)
image_preparer convert photo.png photo.webp --to webp --no-lossy

# Convert JPG to PNG
image_preparer convert photo.jpg photo.png --to png

# Batch convert directory
image_preparer convert ./photos ./output --to webp -r

# Auto-detect output format from extension
image_preparer convert input.png output.jpg --to jpg
```

**Supported conversions:**
- PNG → JPG, WebP
- JPG → PNG, WebP
- WebP → PNG, JPG

**Options:**
- `-t, --to <format>` - Target format (png, jpg, jpeg, webp) **[required]**
- `-q, --quality <0-100>` - Quality for lossy formats (default: 80)
- `--no-lossy` - Use lossless compression
- `-r, --recursive` - Process directories
- `--backup` - Create .bak backups

### Inspect Command

Display detailed file metadata without processing.

```bash
# Inspect single file
image_preparer inspect photo.png

# Inspect video
image_preparer inspect video.mp4

# Inspect MP3 tags
image_preparer inspect song.mp3

# Inspect entire directory
image_preparer inspect ./photos -r
```

**Shows:**
- File size and format
- Image: dimensions, color type, chunks
- Video: duration, codecs, bitrate, resolution, fps
- Audio: ID3 tags, versions

### Extract Command

Extract frames from MP4 videos to PNG images.

```bash
# Extract 1 frame per second (default)
image_preparer extract video.mp4 ./frames/

# Extract 2 frames per second
image_preparer extract video.mp4 ./frames/ -f 2

# Extract all frames
image_preparer extract video.mp4 ./frames/ -f 0

# Extract specific rate
image_preparer extract video.mp4 ./output/ -f 0.5  # 1 frame every 2 seconds
```

**Output:**
- Creates `{video_name}_frames/` directory
- Saves as `frame_0001.png`, `frame_0002.png`, etc.
- Preserves original resolution

**Options:**
- `-f, --fps <N>` - Frames per second (default: 1, 0=all frames)

## Quality Guidelines

### Image Quality (-q)

- **90-100**: Minimal quality loss, ~70-80% compression
- **80-85**: Good quality, ~85-90% compression *(recommended)*
- **70-75**: Acceptable quality, ~90-93% compression
- **50-60**: Noticeable loss, ~94-96% compression
- **0-40**: Heavy loss, ~97-99% compression

### Video Quality (-q)

- **90-100**: Near-lossless, ~70-80% reduction
- **70-80**: Good quality, ~85-92% reduction *(recommended)*
- **50-60**: Medium quality, ~94-96% reduction
- **0-40**: Low quality, ~97-99% reduction

### Speed (-s)

- **1-2**: Very slow, best compression
- **3-4**: Medium speed *(default)*
- **7-10**: Fast, larger files

## Examples

### Optimize photos for web

```bash
image_preparer compress ./photos -r -q 85 --strip all

# Resize for web thumbnails
image_preparer compress ./photos -r -q 80 --width 400 --height 300
```

### Compress videos for storage

```bash
image_preparer compress ./videos -r -q 70 -s 3
```

### Convert images to WebP

```bash
image_preparer convert ./photos ./output --to webp -r -q 80
```

### Create video thumbnails

```bash
image_preparer extract video.mp4 ./thumbs/ -f 0.2
```

### Strip sensitive metadata

```bash
image_preparer compress ./music -r --strip all --no-lossy
```

### Batch convert PNG to JPG

```bash
image_preparer convert ./images ./output --to jpg -r -q 85
```

## Global Options

Available for all commands:

- `-v, --verbose` - Verbose output (shows debug info)
- `-h, --help` - Show help for command
- `-V, --version` - Show version

## Output

After processing, you'll see a summary:

```
Found 5 file(s) to process.
 [████████████████████████████████████████] 5/5 Done!

--- Summary ---
Files processed: 5 | Errors: 0
Total: 52.3 MB → 8.1 MB (84.5% reduction)
```

## Supported Formats

| Format | Extensions | Compress | Convert | Metadata | Extract |
|--------|-----------|----------|---------|----------|---------|
| PNG | `.png` | ✅ | ✅ | ✅ | - |
| WebP | `.webp` | ✅ | ✅ | ✅ | - |
| JPEG | `.jpg`, `.jpeg` | ✅ | ✅ | ✅ | - |
| MP3 | `.mp3` | - | - | ✅ | - |
| MP4 | `.mp4`, `.m4v`, `.m4a` | ✅ | - | ✅ | ✅ |

## Performance

- **Parallel processing**: Utilizes all CPU cores
- **Memory usage**: Proportional to file size
- **Speed**:
  - PNG/WebP: ~1-5s per image
  - MP3: <0.1s per file
  - MP4: ~1-10s per file (depends on length)

## Troubleshooting

### Command not found

If `image_preparer` is not found after install:

```bash
# Check PATH
echo $PATH | grep cargo

# Reinstall
cargo install --path . --force
```

### ffmpeg not found

For MP4 processing:

```bash
# macOS
brew install ffmpeg

# Linux
apt install ffmpeg

# Verify
ffmpeg -version
```

### Out of memory

For very large files:
- Process files individually
- Use `--no-lossy` mode
- Reduce batch size

## Development

```bash
# Build
cargo build --release

# Run tests
cargo test

# Install locally
cargo install --path .

# Update after changes
cargo install --path . --force
```

See [CLAUDE.md](./CLAUDE.md) for development guide.

## License

GPL-3.0-or-later

## Documentation

### CLI Documentation
- **User Guide**: [README.md](./README.md) (this file)
- **Installation**: [INSTALL.md](./INSTALL.md)
- **Development**: [CLAUDE.md](./CLAUDE.md)

### Workspace Documentation
- **Workspace Overview**: [../README.md](../README.md)
- **Server API**: [../server/README.md](../server/README.md)

## Related Projects

- **Web Server**: HTTP API for remote processing (see `../server/`)
- Both CLI and server share the same processing logic

---

**Made with ❤️ and Rust**
