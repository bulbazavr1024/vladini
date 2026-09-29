# Vladini - AI Context & Development Guide

> **Purpose**: This file provides context for AI assistants working on this project.
> **Last Updated**: 2026-09-29

## Project Overview

**Vladini** is a workspace with a single member, the **CLI tool** (`cli/`): a command-line utility for local file processing. The crate is both a library and a binary.

- **Language**: Rust (Edition 2021)
- **License**: GPL-3.0-or-later
- **Primary Use**: Batch processing of media files with configurable quality/speed trade-offs
- **CLI Structure**: Subcommand-based (compress, convert, inspect, extract)

## Workspace Structure

```
vladini/          # Workspace root
├── Cargo.toml                     # Workspace configuration (single member: cli)
├── README.md                      # Overall documentation
│
└── cli/                           # This directory
    ├── src/
    │   ├── lib.rs                # Library exports
    │   ├── main.rs               # CLI binary entry point
    │   ├── cli.rs                # Subcommand definitions
    │   ├── config.rs             # ProcessingConfig, StripMode
    │   ├── converter.rs          # Format conversion
    │   ├── error.rs              # ProcessingError
    │   ├── format.rs             # ImageFormat
    │   ├── pipeline.rs           # Processor dispatcher
    │   ├── processor/            # Format processors
    │   └── ...
    ├── Cargo.toml                # CLI dependencies
    ├── README.md                 # CLI documentation
    ├── INSTALL.md                # Installation guide
    └── CLAUDE.md                 # This file
```

**Critical Design**: The crate is both a library and a binary. Processing logic lives in the library (`src/lib.rs`); `src/main.rs` uses it as `vladini::...`, and library modules refer to each other as `crate::...`. There is no separate core crate.

## Architecture

### Core Pattern: Processor Pipeline

The project uses a **processor pipeline pattern** where format-specific processors implement the `ImageProcessor` trait:

```
Input → Config → Pipeline → Format Detection → Processor → Output
  ↓                                                            ↓
CLI Subcommand                                        CLI Output
```

**Key Components**:
- `src/lib.rs` - Library exports (pipeline, processors, config, etc.)
- `src/main.rs` - CLI entry point, subcommand routing
- `src/cli.rs` - Clap subcommand definitions
- `src/pipeline.rs` - Dispatches files to processors
- `src/processor/mod.rs` - `ImageProcessor` trait
- `src/processor/{format}.rs` - Format-specific implementations
- `src/converter.rs` - Format conversion logic
- `src/config.rs` - Shared `ProcessingConfig` + `StripMode`

### Processor Interface

```rust
pub trait ImageProcessor: Send + Sync {
    fn supported_formats(&self) -> &[ImageFormat];
    fn process(&self, input: &[u8], config: &ProcessingConfig) -> Result<Vec<u8>, ProcessingError>;
}
```

## CLI Structure (Subcommands)

The CLI uses clap subcommands for different operations:

```rust
pub enum Command {
    Compress { /* compress options */ },
    Convert { /* convert options */ },
    Inspect { /* inspect options */ },
    Extract { /* extract options */ },
}
```

### Command Usage

```bash
# Compress images/videos
vladini compress <input> [output] [options]

# Convert between formats
vladini convert <input> [output] [--to <format>] [options]

# Inspect metadata
vladini inspect <input> [options]

# Extract video frames
vladini extract <input> <output> [options]
```

## Supported Formats

### ✅ PNG (`src/processor/png.rs`)
- **Compression**: Lossy via imagequant → Lossless via oxipng
- **Resize**: Supported (Lanczos3 filter, applied before compression)
- **Metadata**: Delegated to oxipng's `StripChunks`
- **Dependencies**: `image`, `imagequant`, `lodepng`, `oxipng`
- **Typical reduction**: 50-90%
- **Commands**: compress, convert, inspect

### ✅ WebP (`src/processor/webp.rs`)
- **Compression**: Lossy/Lossless via webp crate
- **Resize**: Supported (Lanczos3 filter, applied before encoding)
- **Metadata**: Custom RIFF chunk filtering (EXIF, XMP, ICCP)
- **StripMode mapping**:
  - `All`: Keep only VP8/VP8L/ALPH
  - `Safe`: Add VP8X/ANIM/ANMF
  - `None`: Keep all
- **Dependencies**: `webp`, `image`
- **Typical reduction**: 40-80%
- **Commands**: compress, convert, inspect

### ✅ JPEG (`src/processor/jpg.rs`)
- **Compression**: Re-encode via image crate JPEG encoder with quality/resize
- **Conversion**: Supported as target/source format
- **Resize**: Supported (Lanczos3 filter)
- **Note**: No alpha channel support (converts to RGB)
- **Quality**: Configurable 0-100
- **Commands**: compress, convert, inspect

### ✅ MP3 (`src/processor/mp3.rs`)
- **Compression**: N/A (already compressed)
- **Metadata**: ID3 tag removal (v1 and v2)
- **StripMode mapping**:
  - `All`: Remove all ID3 tags
  - `Safe`: Keep basic tags (TIT2, TPE1, TALB, etc.), remove unsafe (APIC, COMM, PRIV)
  - `None`: Return unchanged
- **Dependencies**: `id3`
- **API Notes**: Use `Tag::read_from2()` not deprecated `read_from()`
- **Commands**: compress, convert (via ffmpeg), inspect

### ✅ MP4 (`src/processor/mp4.rs`)
- **Compression**: Requires **ffmpeg** (system dependency)
- **Lossy mode**: Re-encode with H.264 + quality/speed mapping
  - Quality (0-100) → CRF (18-35)
  - Speed (1-10) → ffmpeg presets (veryslow to ultrafast)
- **Lossless mode**: Copy streams, strip metadata only
- **Frame extraction**: Outputs to `{video_name}_frames/` directory
  - FPS=0 extracts all frames
  - FPS=N extracts N frames per second
- **Dependencies**: `mp4` (parsing), `ffmpeg` (processing)
- **Typical reduction**: 70-96% (lossy), ~0.5% (lossless)
- **System requirement**: `ffmpeg` must be installed
- **Commands**: compress, convert (via ffmpeg), inspect, extract

## Format Conversion (`src/converter.rs`)

The converter module handles image and audio/video conversion. The target format picks the path. `--to` is optional and defaults to the output file's extension (error if neither `--to` nor an output file with an extension is given; `--to` is still needed for directory output):

- `png|jpg|jpeg|webp` → `convert_image` (`image` crate; input must be an image)
- any other target (mp3, wav, flac, ogg, opus, m4a, aac, mp4, webm, mkv, mov, avi, gif, ...) → `convert_media` (ffmpeg)

```rust
pub enum ConvertFormat { Png, Jpg, Webp }

pub fn convert_image(
    input: &[u8],
    target_format: ConvertFormat,
    config: &ProcessingConfig,
) -> Result<Vec<u8>, ProcessingError>

pub fn convert_media(
    input: &Path,
    output: &Path,
    config: &ProcessingConfig,
) -> Result<(), ProcessingError>
```

**Image conversions** (`convert_image`):
- PNG → JPG, WebP
- JPG → PNG, WebP
- WebP → PNG, JPG
- Uses `image` crate for loading/encoding
- PNG: Standard encoding
- JPG: JPEG encoder with quality
- WebP: webp crate with lossy/lossless

**Audio/video conversions** (`convert_media`):
- ffmpeg picks container/codec from the output extension, so anything the installed ffmpeg supports works
- Audio-only targets drop the video stream (`-vn`). Quality: `-b, --bitrate 192k` is an explicit bitrate and overrides `-q`. Without it, `-q` maps to:
  - mp3/m4a/aac/opus/wma/ac3: bitrate 32–256 kbps, capped at the source bitrate (re-encoding never inflates the file)
  - ogg/oga/mka (Vorbis): VBR quality `-q:a` = q/10 (0–10), because vorbis rejects fixed bitrates on mono/low settings
  - wav/flac/aiff (lossless): ignored
- Video targets map `-q` to CRF 18–35 (same formula as MP4 compression)
- Metadata: `--strip <all|safe|none>`, default `none`, so audio/video tags (artist, title, ...) are kept; `--strip all` removes them (`-map_metadata -1`). Image targets are re-encoded and never carry metadata
- `--width/--height`: images are resized by the `image` crate (same as compress); video/gif via ffmpeg scale (a missing side keeps aspect ratio, even size)
- `--start/--end`: trim audio/video (`--start 1:30 --end 2:45`); `--end` is a position in the source, not a duration. Error with png/jpg/webp targets
- GIF target: palette generated from the clip (palettegen/paletteuse) for proper colors
- `-j, --jobs N`: max files converted in parallel (default: CPU cores); each ffmpeg is already multi-threaded, so lower it for folders of videos
- Output paths: a directory input is mirrored under the output dir (subfolders kept, like compress); the output dir is created if missing
- Directory inputs are filtered by `MEDIA_EXTENSIONS` (png jpg jpeg webp gif bmp tif tiff avif mp3 wav flac ogg oga opus m4a aac aiff wma ac3 mka mp4 m4v mov mkv webm avi flv wmv mpg mpeg 3gp), then only files that can become the target are picked: audio target skips images (cover art), image target (png/jpg/webp/bmp/tiff/avif) takes only images, gif target skips audio, video targets take everything
- A file already in the target format with no output given (e.g. `./music --to mp3` containing mp3s) is skipped; the summary reads "Files processed: N | Skipped: M | Errors: K"
- If several inputs would produce the same output (clip.mkv + clip.mp3 → clip.mp4), all of them fail with "several inputs convert to ... - convert them separately" instead of overwriting each other
- Requires ffmpeg

## Main.rs Structure

The main.rs is organized into handler functions:

- `main()` - Parse CLI, route to subcommand handlers
- `handle_compress()` - Compression logic with pipeline
- `handle_convert()` - Format conversion logic
- `handle_inspect()` - Metadata display
- `handle_extract()` - Frame extraction for MP4

Each handler:
1. Collects files
2. Creates progress bar
3. Processes in parallel (rayon)
4. Reports results

## Configuration & CLI

### Subcommands

```rust
compress [OPTIONS] <INPUT> [OUTPUT]
  -q, --quality <0-100>      # Default: 80
  -s, --speed <1-10>         # Default: 3
  --no-lossy                 # Lossless only
  --strip <all|safe|none>    # Default: all
  -r, --recursive
  --backup
  --dry-run
  --width <pixels>           # Resize width (preserves aspect ratio if height omitted)
  --height <pixels>          # Resize height (preserves aspect ratio if width omitted)

convert [OPTIONS] <INPUT> [OUTPUT]
  -t, --to <format>          # png|jpg|jpeg|webp or any ffmpeg extension; default: output file's extension
  -q, --quality <0-100>      # Default: 80 (audio bitrate / video CRF for ffmpeg targets)
  -b, --bitrate <e.g. 192k>  # Audio bitrate, overrides -q
  --no-lossy
  --strip <all|safe|none>    # Default: none (audio/video tags kept)
  --width <pixels>           # Resize (images: image crate, video/gif: ffmpeg scale)
  --height <pixels>
  --start <pos>              # Trim audio/video, e.g. 1:30 (error for png/jpg/webp)
  --end <pos>                # Position in the source, e.g. 2:45 (not a duration)
  -j, --jobs <N>             # Max parallel files (default: CPU cores)
  -r, --recursive
  --backup

inspect [OPTIONS] <INPUT>
  -r, --recursive

extract [OPTIONS] <INPUT> <OUTPUT>
  -f, --fps <N>              # Default: 1, 0=all
```

### Global Options

```rust
-v, --verbose    # Enable debug logging
```

### StripMode Interpretation

Different formats interpret `StripMode` differently:
- **PNG**: Maps directly to oxipng's `StripChunks` enum
- **WebP**: Custom RIFF chunk filtering
- **MP4**: ffmpeg `-map_metadata -1`
- **MP3**: Custom safe frame filtering

## Development Patterns

### Adding New Format Support

1. Add format to `ImageFormat` enum in `src/format.rs`
2. Create processor in `src/processor/<format>.rs`
3. Implement `ImageProcessor` trait + `inspect_<format>()` function
4. Add `pub mod <format>;` to `src/processor/mod.rs`
5. Register processor in `handle_compress()` pipeline
6. Add inspect handler in `handle_inspect()`
7. Add dependencies to `Cargo.toml`
8. Update `CLAUDE.md` (this file) and auto memory

### Adding New Subcommand

1. Add variant to `Command` enum in `src/cli.rs`
2. Create handler function in `src/main.rs`
3. Route in `main()` match statement
4. Update documentation (README.md, CLAUDE.md)

### Code Style Guidelines

- **Error Handling**: Use `ProcessingError::{Decode, Encode, Quantize, Optimize}`
- **Logging**: Use `log::{debug, info, warn, error}` with `-v` flag
- **Parallelization**: Use rayon for parallel file processing
- **Progress**: Use indicatif for progress bars
- **Testing**: Manual testing with synthetic files (no automated tests yet)

### Important API Notes

**id3 crate**:
- Use `Tag::read_from2()` not deprecated `read_from()`
- `tag.version()` returns enum (`Id3v22`, `Id3v23`, `Id3v24`), not tuple
- Synchsafe integers: 7 bits per byte for size encoding

**mp4 crate**:
- `ftyp` is a field, not method (`mp4.ftyp` not `mp4.ftyp()`)
- `bitrate()` returns `u32`, not `Option<u32>`
- `track_type()` returns `Result<TrackType, Error>`, must unwrap
- `frame_rate()` returns `f64`, not `Option<f64>`

**ffmpeg integration**:
- Check availability with `ffmpeg -version`
- Use temporary files in `std::env::temp_dir()`
- Always clean up temp files after processing
- Log ffmpeg stderr on failure

**image crate**:
- Import `GenericImageView` for `.dimensions()`
- JPEG encoder needs `mut` for `.encode()`
- Use `.to_rgba8()` before WebP encoding

## System Requirements

- **Rust**: Edition 2021, version 1.70+
- **ffmpeg**: Required for MP4 processing and audio/video conversion
  - macOS: `brew install ffmpeg`
  - Linux: `apt install ffmpeg`
- **Memory**: Scales with file size (processes in RAM)

## Common Pitfalls & Solutions

### 🚫 MP4 Processing Errors
- **Problem**: Aggressive box filtering can corrupt MP4 files
- **Solution**: Only reorder boxes for fast start, never remove critical boxes
- **Safe approach**: Use ffmpeg for all MP4 modifications

### 🚫 CLI Structure Changes
- **Problem**: Old flat CLI vs new subcommand structure
- **Solution**: Always use subcommands: `vladini compress` not `vladini`
- **Migration**: Update docs and examples when changing CLI

### 🚫 Path Type Mismatches
- **Problem**: `Option<&PathBuf>` vs `Option<&Path>`
- **Solution**: Use `.as_deref()` to convert: `output.as_deref()`

### 🚫 Parallel Processing Issues
- **Problem**: Progress bar and file writes can conflict
- **Solution**: Use `Mutex<Report>` for thread-safe reporting

### 🚫 Large File Memory Usage
- **Problem**: Loading entire file into `Vec<u8>` can exhaust memory
- **Solution**: Currently not addressed - future: streaming or size limits

## Metadata Inspection

The `inspect` command provides detailed format-specific metadata viewing:

- **PNG**: Chunks with sizes, types (critical/ancillary), IHDR/tEXt/pHYs data
- **WebP**: RIFF structure, VP8/VP8L bitstreams, canvas dimensions, format flags
- **MP3**: ID3v2 frames, ID3v1 tags, safe/unsafe markers, automatic file path detection
- **MP4**: File type, tracks (codec, bitrate, dimensions, fps), duration, fast start status

## Future Improvements

### Planned
- [ ] HEIC/HEIF support
- [ ] GIF optimization
- [ ] Streaming processing for large files
- [ ] Progress estimation for MP4 re-encoding
- [ ] Automated tests
- [ ] Benchmarking suite

### Under Consideration
- [ ] MP4 metadata stripping without ffmpeg (complex, risky)
- [ ] GPU-accelerated video encoding
- [ ] WebAssembly support
- [ ] GUI wrapper

## Project Structure (CLI)

This is the CLI subproject within the workspace. See "Workspace Structure" section above for the full layout.

```
cli/                      # This directory
├── src/
│   ├── main.rs           # CLI binary entry point
│   ├── lib.rs            # Library exports
│   ├── cli.rs            # Clap subcommand definitions
│   ├── config.rs         # ProcessingConfig, StripMode
│   ├── converter.rs      # Format conversion logic
│   ├── error.rs          # ProcessingError enum
│   ├── format.rs         # ImageFormat enum
│   ├── io.rs             # File I/O utilities
│   ├── pipeline.rs       # Processor dispatcher
│   ├── report.rs         # Processing statistics
│   └── processor/
│       ├── mod.rs        # ImageProcessor trait
│       ├── png.rs        # PNG processor + inspect
│       ├── webp.rs       # WebP processor + inspect
│       ├── mp3.rs        # MP3 processor + inspect
│       └── mp4.rs        # MP4 processor + inspect + extract
├── Cargo.toml            # CLI dependencies
├── CLAUDE.md             # This file (AI context)
├── README.md             # CLI user documentation
└── INSTALL.md            # Installation guide
```

**Important**: The CLI is configured as both a library (`[lib]`) and a binary (`[[bin]]`) in Cargo.toml.

## Dependencies Summary

```toml
# CLI & Utilities
clap = "4"              # Argument parsing (with subcommands)
anyhow = "1"            # Error handling
thiserror = "2"         # Error derive macros
log = "0.4"             # Logging facade
env_logger = "0.11"     # Logger implementation
walkdir = "2"           # Directory traversal
indicatif = "0.17"      # Progress bars
rayon = "1"             # Parallelization

# Image Processing
image = "0.25"          # Image loading/encoding
imagequant = "4"        # Color quantization
lodepng = "3"           # PNG encoding
oxipng = "10"           # PNG optimization
webp = "0.3"            # WebP encoding/decoding

# Audio/Video Processing
id3 = "1.14"            # MP3 ID3 tags
mp4 = "0.14"            # MP4 container parsing
# + ffmpeg (system dependency)
```

## Quick Reference Commands

```bash
# Build CLI (from workspace root)
cargo build --release --bin vladini

# Build CLI (from cli directory)
cd cli
cargo build --release

# Install CLI globally (from workspace root)
cargo install --path cli

# Install CLI globally (from cli directory)
cd cli
cargo install --path .

# Update after changes
cargo install --path cli --force  # From workspace root
cargo install --path . --force     # From cli directory

# Run CLI
vladini compress photo.png -q 80
vladini convert image.png --to jpg
vladini convert clip.webm clip.mp3
vladini inspect file.mp4
vladini extract video.mp4 ./frames/

# Help
vladini --help
vladini compress --help
vladini convert --help
```

## Notes for AI Assistants

- Always read this file at the start of a new session
- This is a workspace with a single member: the CLI (this directory), which is both a library and a binary
- Update this file when adding features or changing CLI structure
- Keep the "Last Updated" date current
- Maintain consistency with auto memory (`~/.claude/projects/.../memory/MEMORY.md`)
- When debugging, check "Common Pitfalls" section first
- For MP4 issues, verify ffmpeg is installed and accessible
- CLI uses subcommands - never suggest flat command structure

## CLI Migration Guide

**Old (deprecated)**:
```bash
vladini file.png --convert-to jpg
vladini file.png --extract-frames
vladini file.png --inspect
```

**New (current)**:
```bash
vladini convert file.png --to jpg
vladini extract file.mp4 ./frames/
vladini inspect file.png
vladini compress file.png -q 80
```

## Library Structure

The CLI exports its functionality via `src/lib.rs`:
- `config::*` - ProcessingConfig, StripMode
- `pipeline::Pipeline` - Main processor dispatcher
- `processor::*` - All format processors (PNG, WebP, MP3, MP4)
- `converter::*` - Format conversion functions (`convert_image`, `convert_media`)
- `format::ImageFormat` - Format detection
- `error::ProcessingError` - Error types

`src/main.rs` imports these as `vladini::...`; inside the library use `crate::...`.

## Contact & Resources

- **Author**: pavelagejkin
- **Claude Auto Memory**: `~/.claude/projects/.../memory/MEMORY.md`

---

**Remember**: This tool prioritizes **correctness** over speed. Never sacrifice data integrity for performance.

**CLI Philosophy**: Clear subcommands over complex flags. Each operation is a distinct command with focused options.
