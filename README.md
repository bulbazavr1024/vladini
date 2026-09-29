# Vladini Workspace

A Rust workspace containing a command-line utility for image/video/audio processing.

## Project Structure

```
vladini/
├── Cargo.toml          # Workspace configuration
├── README.md           # This file
└── cli/                # CLI tool (binary + library)
    ├── src/
    ├── Cargo.toml
    ├── README.md       # CLI documentation
    ├── INSTALL.md      # Installation guide
    └── CLAUDE.md       # AI development guide
```

## Features

### Supported Formats

| Format | Compress | Convert | Inspect | Extract |
|--------|----------|---------|---------|---------|
| PNG    | ✅       | ✅      | ✅      | -       |
| JPG    | ✅       | ✅      | ✅      | -       |
| WebP   | ✅       | ✅      | ✅      | -       |
| MP3    | ✅*      | ✅**    | ✅      | -       |
| MP4    | ✅       | ✅**    | ✅      | ✅      |

*MP3 compression = metadata stripping only

**Audio/video conversion is done by ffmpeg (see Convert below)

### Operations

- **Compress**: Reduce file size with lossy/lossless algorithms + optional resize
- **Convert**: Transform between image formats (PNG ↔ JPG ↔ WebP), or convert audio/video (e.g. `--to mp3`, `--to mp4`) via ffmpeg. Any target the installed ffmpeg supports works. Supports resizing, trimming (`--start/--end`), audio bitrate (`-b`) and parallel jobs (`-j`)
- **Inspect**: View detailed metadata
- **Extract**: Extract video frames to PNG images

## Quick Start

### Install (macOS, Apple Silicon)

```bash
brew tap bulbazavr1024/vladini https://github.com/bulbazavr1024/vladini
brew install vladini          # also installs ffmpeg
brew upgrade vladini          # update to the latest release
```

From source instead: `cargo install --path cli`.

### Usage

```bash
vladini compress photo.png -q 80
vladini convert image.png --to webp
vladini convert clip.webm clip.mp3
vladini inspect video.mp4
vladini extract video.mp4 ./frames/
```

See [CLI README](cli/README.md) for complete documentation.

## Building

```bash
# Build
cargo build --release

# Run tests (when available)
cargo test
```

## System Requirements

- **Rust**: Edition 2021, version 1.70+
- **ffmpeg**: Required for MP4 processing and audio/video conversion
  - macOS: `brew install ffmpeg`
  - Linux: `apt install ffmpeg`
- **Memory**: Scales with file size (processes in RAM)

## Architecture

The workspace has a single member, the `cli` crate, which is both a library and a binary:

```
CLI (vladini)
├── Library: src/lib.rs   (pipeline, processors, converter, config)
└── Binary: src/main.rs   (uses the library as vladini::...)
```

### Library Components

- **Pipeline**: Format detection and routing
- **Processors**: Format-specific implementations (PNG, WebP, MP3, MP4)
- **Converter**: Image format conversion (`image` crate) and audio/video conversion (ffmpeg)
- **Config**: Processing configuration

### Binary (main.rs)

- Subcommand parsing (clap)
- Progress bars (indicatif)
- Parallel file processing (rayon)
- File I/O utilities

## Development

### Adding Format Support

1. Add format to `cli/src/format.rs`
2. Create processor in `cli/src/processor/<format>.rs`
3. Implement `ImageProcessor` trait
4. Register in CLI handlers
5. Test with the CLI
6. Update documentation

See [CLAUDE.md](cli/CLAUDE.md) for detailed development guide.

### Code Organization

- **cli/src/**: Binary and library code
  - `main.rs`: CLI entry point
  - `lib.rs`: Library exports
  - `config.rs`: Processing configuration
  - `converter.rs`: Format conversion (images and audio/video)
  - `error.rs`: Error types
  - `format.rs`: Format detection
  - `pipeline.rs`: Processing pipeline
  - `processor/`: Format processors

## Typical Workflows

### Batch Image Optimization

```bash
# Compress all PNGs in a directory
vladini compress ./photos/ ./optimized/ -q 85 -r

# Resize and compress for web
vladini compress ./photos/ -r -q 80 --width 1200

# Resize to exact dimensions
vladini compress photo.png --width 800 --height 600

# Convert all JPGs to WebP
for f in *.jpg; do
  vladini convert "$f" --to webp
done
```

### Video Processing

```bash
# Compress video with high quality
vladini compress video.mp4 output.mp4 -q 90 -s 2

# Extract frames (1 per second)
vladini extract video.mp4 ./frames/ --fps 1

# Extract all frames
vladini extract video.mp4 ./frames/ --fps 0
```

### Audio/Video Conversion

```bash
# Extract audio from a video (format from the output extension)
vladini convert clip.webm clip.mp3

# Folder of FLAC to MP3 at 192 kbps
vladini convert ./music ./out --to mp3 -b 192k

# Cut a fragment to MP3
vladini convert talk.mp4 fragment.mp3 --start 1:30 --end 2:45

# Video to GIF
vladini convert clip.mp4 clip.gif --width 480 --start 5 --end 8

# Photos folder to WebP at 1600px width
vladini convert ./photos ./out --to webp --width 1600
```

## Performance

### Compression Ratios

Typical size reductions:
- **PNG**: 50-90% (lossy + lossless)
- **WebP**: 40-80% (lossy)
- **MP4**: 70-95% (lossy re-encoding)
- **MP3**: 0-5% (metadata removal only)

### Processing Speed

- **Images**: 1-10 images/second (depends on size and quality)
- **Videos**: Slower than real-time (depends on resolution and speed preset)
- **Parallel processing**: Scales with CPU cores

## Releasing

Bump `version` in `cli/Cargo.toml`, commit, then push a matching tag:

```bash
git tag v0.2.1 && git push origin main v0.2.1
```

GitHub Actions (`.github/workflows/release.yml`) builds the arm64 binary, attaches it to a GitHub Release
and commits the updated Homebrew formula (`Formula/vladini.rb`) to `main` — run `git pull` afterwards.
Installed copies pick it up with `brew upgrade vladini`.

## License

GPL-3.0-or-later

## Documentation

- [CLI README](cli/README.md) - Command-line usage
- [CLI INSTALL](cli/INSTALL.md) - Installation guide
- [CLAUDE.md](cli/CLAUDE.md) - AI development context

## Contributing

This is a personal project, but suggestions are welcome.

## Known Limitations

- MP4 processing and audio/video conversion require ffmpeg
- Large files loaded entirely into RAM
- No streaming processing yet

## Troubleshooting

### MP4 Processing Fails

- Ensure ffmpeg is installed: `ffmpeg -version`
- Check file permissions
- Verify MP4 is not corrupted: `ffmpeg -i file.mp4`

### Out of Memory

- Reduce image size before processing
- Process files one at a time (not in parallel)
- Use lossless mode (`--no-lossy`)

### Dependencies Won't Build

- Update Rust: `rustup update`
- Clean build: `cargo clean && cargo build`
- Check ffmpeg: `brew install ffmpeg` (macOS)

## Changelog

### v0.2.0 (2026-09-29)
- Renamed the tool to `vladini`
- Homebrew install/upgrade from GitHub Releases built by CI
- Removed the HTTP server (`server/` crate and API); the project is now CLI-only
- Merged the `core` crate into `cli` (single workspace member, code imports `vladini::...`)
- `convert` now handles audio/video via ffmpeg (e.g. `convert clip.webm clip.mp3`); `--to` is optional and defaults to the output file's extension
- `convert` options: `--strip` (default `none`, tags kept), `-b/--bitrate`, `--width/--height`, `--start/--end` trimming, `-j/--jobs`, GIF output; directories are mirrored under the output dir and already-converted files are skipped

### v0.1.0 (2026-02-06)
- Initial workspace structure
- CLI tool with compress, convert, inspect, extract commands
- Web server with REST API
- Support for PNG, WebP, MP3, MP4 formats
- Parallel processing
- Metadata stripping
- Format conversion (PNG/JPG/WebP)
