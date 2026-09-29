# Image Preparer CLI

Command-line tool for compressing images/videos, converting between formats, and stripping metadata.

> **Note**: This is part of the Image Preparer workspace. For workspace documentation, see `../README.md`.

## Features

- ✅ **PNG** - Lossy/Lossless compression (50-90% reduction)
- ✅ **WebP** - Lossy/Lossless compression (40-80% reduction)
- ✅ **JPEG** - Compression + format conversion
- ✅ **MP3** - Metadata stripping (ID3 tags)
- ✅ **MP4** - Video compression (70-96% reduction) + Frame extraction
- 🔄 **Format conversion** - PNG ↔ JPG ↔ WebP, plus audio/video (mp3, wav, flac, mp4, webm, ...) via ffmpeg
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
- **ffmpeg** (required for MP4 processing and audio/video conversion)
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
- `convert` - Convert images, audio and video between formats
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

Convert images between PNG, JPG, and WebP formats, or convert audio/video to any format ffmpeg supports.

```bash
# Convert PNG to JPG
image_preparer convert photo.png photo.jpg --to jpg

# Target format is taken from the output extension when --to is omitted
image_preparer convert clip.webm clip.mp3

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

# Photos folder to WebP, resized to 1600px width
image_preparer convert ./photos ./out --to webp --width 1600

# Extract audio from a video
image_preparer convert clip.webm --to mp3

# Folder of FLAC to MP3 at 192 kbps, output into ./out
image_preparer convert ./music ./out --to mp3 -b 192k

# Cut a fragment (1:30 to 2:45 of the source) to MP3
image_preparer convert talk.mp4 fragment.mp3 --start 1:30 --end 2:45

# Video to GIF, 480px wide, seconds 5-8
image_preparer convert clip.mp4 clip.gif --width 480 --start 5 --end 8

# Convert video to MP4 with output path
image_preparer convert video.mkv out.mp4 --to mp4

# Folder of videos, at most 2 files at a time
image_preparer convert ./videos ./out --to webm -j 2
```

**Options:**

| Option | Description |
|--------|-------------|
| `-t, --to <format>` | Target: png, jpg, jpeg, webp, or any ffmpeg-supported audio/video extension. Optional: defaults to the output file's extension. Error if neither `--to` nor an output file with an extension is given; still needed for directory output |
| `-q, --quality <0-100>` | Quality for lossy formats (default: 80). Audio bitrate / video CRF for ffmpeg targets (see below) |
| `-b, --bitrate <rate>` | Explicit audio bitrate, e.g. `192k`. Overrides `-q` |
| `--no-lossy` | Use lossless compression |
| `--strip <all\|safe\|none>` | Metadata stripping (default: `none`). Audio/video tags (artist, title, ...) are kept by default; `--strip all` removes them. Image targets are re-encoded and never carry metadata |
| `--width <pixels>` | Resize width (aspect ratio preserved if height omitted) |
| `--height <pixels>` | Resize height (aspect ratio preserved if width omitted) |
| `--start <pos>` | Trim audio/video start, e.g. `90` or `1:30`. Error with png/jpg/webp targets |
| `--end <pos>` | Trim end, e.g. `2:45`. A position in the source, not a duration. Error with png/jpg/webp targets |
| `-j, --jobs <N>` | Max files converted in parallel (default: CPU cores). Lower it for folders of videos, since each ffmpeg is already multi-threaded |
| `-r, --recursive` | Process directories |
| `--backup` | Create .bak backups |

**Supported conversions:**
- PNG → JPG, WebP
- JPG → PNG, WebP
- WebP → PNG, JPG
- Audio/video → any format ffmpeg supports (mp3, wav, flac, ogg, opus, m4a, aac, mp4, webm, mkv, mov, avi, gif, ...)

**Image vs. audio/video targets:**
- `png|jpg|jpeg|webp` - handled by the `image` crate; input must be an image
- Any other target - handed to ffmpeg, which picks container/codec from the extension (requires ffmpeg)
- Audio-only targets drop the video stream. Without `-b`, quality comes from `-q`:
  - mp3, m4a, aac, opus, wma, ac3: bitrate 32–256 kbps
  - ogg, oga, mka (Vorbis): VBR quality `-q:a` = q/10 (0–10)
  - wav, flac, aiff (lossless): ignore quality
- Video targets map `-q` to CRF 18–35 (same formula as MP4 compression)
- `--width/--height`: images are resized by the `image` crate (same as `compress`); video/gif via ffmpeg scale (a missing side keeps aspect ratio, even size)
- GIF targets generate a palette from the clip (palettegen/paletteuse) for proper colors

**Batch behavior:**
- A directory input is mirrored under the output directory (subfolders kept, like `compress`); the output directory is created if missing
- Directory inputs pick only files that can become the target: audio targets skip images (cover art), image targets (png/jpg/webp/bmp/tiff/avif) take only images, gif targets skip audio, video targets take everything
- A file already in the target format with no output given (e.g. `./music --to mp3` containing mp3s) is skipped; the summary reads `Files processed: N | Skipped: M | Errors: K`
- If several inputs would produce the same output (`clip.mkv` + `clip.mp3` → `clip.mp4`), all of them fail with "several inputs convert to ... - convert them separately" instead of overwriting each other

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

# Resize to 1600px width while converting
image_preparer convert ./photos ./out --to webp --width 1600
```

### Convert audio and video

```bash
# webm to mp3
image_preparer convert clip.webm --to mp3

# Folder of FLAC to MP3 at 192k into ./out
image_preparer convert ./music ./out --to mp3 -b 192k

# Cut a fragment to mp3
image_preparer convert talk.mp4 fragment.mp3 --start 1:30 --end 2:45

# Video to GIF
image_preparer convert clip.mp4 clip.gif --width 480 --start 5 --end 8
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
Files processed: 5 | Skipped: 0 | Errors: 0
Total: 52.3 MB → 8.1 MB (84.5% reduction)
```

## Supported Formats

| Format | Extensions | Compress | Convert | Metadata | Extract |
|--------|-----------|----------|---------|----------|---------|
| PNG | `.png` | ✅ | ✅ | ✅ | - |
| WebP | `.webp` | ✅ | ✅ | ✅ | - |
| JPEG | `.jpg`, `.jpeg` | ✅ | ✅ | ✅ | - |
| MP3 | `.mp3` | - | ✅* | ✅ | - |
| MP4 | `.mp4`, `.m4v`, `.m4a` | ✅ | ✅* | ✅ | ✅ |

*Audio/video conversion is done by ffmpeg and works for any format the installed ffmpeg supports.

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

For MP4 processing and audio/video conversion:

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

---

**Made with ❤️ and Rust**
