# Image Preparer Workspace

A Rust workspace containing two projects for image/video/audio processing:

1. **CLI** - Command-line utility for local file processing
2. **Server** - HTTP API server for remote processing

## Project Structure

```
image_preparer_workspace/
├── Cargo.toml          # Workspace configuration
├── README.md           # This file
├── cli/                # CLI tool
│   ├── src/
│   ├── Cargo.toml
│   ├── README.md       # CLI documentation
│   ├── INSTALL.md      # Installation guide
│   └── CLAUDE.md       # AI development guide
└── server/             # Web server
    ├── src/
    ├── Cargo.toml
    └── README.md       # API documentation
```

## Features

### Supported Formats

| Format | Compress | Convert | Inspect | Extract |
|--------|----------|---------|---------|---------|
| PNG    | ✅       | ✅      | ✅      | -       |
| JPG    | ✅       | ✅      | ✅      | -       |
| WebP   | ✅       | ✅      | ✅      | -       |
| MP3    | ✅*      | -       | ✅      | -       |
| MP4    | ✅       | -       | ✅      | ✅      |

*MP3 compression = metadata stripping only

### Operations

- **Compress**: Reduce file size with lossy/lossless algorithms + optional resize
- **Convert**: Transform between image formats (PNG ↔ JPG ↔ WebP)
- **Inspect**: View detailed metadata
- **Extract**: Extract video frames to PNG images

## Quick Start

### CLI Tool

```bash
# Install globally
cargo install --path cli

# Use from anywhere
image_preparer compress photo.png -q 80
image_preparer convert image.png --to webp
image_preparer inspect video.mp4
image_preparer extract video.mp4 ./frames/
```

See [CLI README](cli/README.md) for complete documentation.

### Web Server

```bash
# Start server
cargo run --release --bin server

# Use API
curl -X POST \
  -F "file=@image.png" \
  -F "quality=85" \
  -o compressed.png \
  http://localhost:3000/compress
```

See [Server README](server/README.md) for API documentation.

## Building

```bash
# Build everything
cargo build --release

# Build CLI only
cargo build --release --bin image_preparer

# Build server only
cargo build --release --bin server

# Run tests (when available)
cargo test
```

## System Requirements

- **Rust**: Edition 2021, version 1.70+
- **ffmpeg**: Required for MP4 processing
  - macOS: `brew install ffmpeg`
  - Linux: `apt install ffmpeg`
- **Memory**: Scales with file size (processes in RAM)

## Architecture

The workspace uses a shared library pattern:

```
CLI (image_preparer)
├── Binary: src/main.rs
└── Library: src/lib.rs ───┐
                           │
Server                     │
├── Binary: src/main.rs    │
└── Uses library: ─────────┘
```

### Shared Components

Both CLI and server use the same processing logic:
- **Pipeline**: Format detection and routing
- **Processors**: Format-specific implementations (PNG, WebP, MP3, MP4)
- **Converter**: Image format conversion
- **Config**: Processing configuration

### CLI-Specific

- Subcommand parsing (clap)
- Progress bars (indicatif)
- Parallel file processing (rayon)
- File I/O utilities

### Server-Specific

- HTTP routing (axum)
- Multipart form handling
- JSON responses
- CORS middleware

## Development

### Adding Format Support

1. Add format to `cli/src/format.rs`
2. Create processor in `cli/src/processor/<format>.rs`
3. Implement `ImageProcessor` trait
4. Register in CLI handlers
5. Test with both CLI and server
6. Update documentation

See [CLAUDE.md](cli/CLAUDE.md) for detailed development guide.

### Code Organization

- **cli/src/**: CLI-specific code and shared library
  - `main.rs`: CLI entry point
  - `lib.rs`: Library exports
  - `processor/`: Format processors
  - `converter.rs`: Format conversion
  - `pipeline.rs`: Processing pipeline
- **server/src/**: Server-specific code
  - `main.rs`: Server entry point
  - `handlers.rs`: API endpoints

## Typical Workflows

### Batch Image Optimization

```bash
# Compress all PNGs in a directory
image_preparer compress ./photos/ ./optimized/ -q 85 -r

# Resize and compress for web
image_preparer compress ./photos/ -r -q 80 --width 1200

# Resize to exact dimensions
image_preparer compress photo.png --width 800 --height 600

# Convert all JPGs to WebP
for f in *.jpg; do
  image_preparer convert "$f" --to webp
done
```

### Video Processing

```bash
# Compress video with high quality
image_preparer compress video.mp4 output.mp4 -q 90 -s 2

# Extract frames (1 per second)
image_preparer extract video.mp4 ./frames/ --fps 1

# Extract all frames
image_preparer extract video.mp4 ./frames/ --fps 0
```

### Web API Integration

```bash
# Compress image via API
curl -X POST \
  -F "file=@photo.png" \
  -F "quality=80" \
  -o compressed.png \
  http://localhost:3000/compress

# Convert format via API
curl -X POST \
  -F "file=@image.png" \
  -F "to=webp" \
  -o output.webp \
  http://localhost:3000/convert
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

## License

GPL-3.0-or-later

## Documentation

- [CLI README](cli/README.md) - Command-line usage
- [CLI INSTALL](cli/INSTALL.md) - Installation guide
- [Server README](server/README.md) - API documentation
- [CLAUDE.md](cli/CLAUDE.md) - AI development context

## Contributing

This is a personal project, but suggestions are welcome.

## Known Limitations

- MP4 processing requires ffmpeg
- Large files loaded entirely into RAM
- No streaming processing yet
- Frame extraction only available in CLI (not API)

## Troubleshooting

### MP4 Processing Fails

- Ensure ffmpeg is installed: `ffmpeg -version`
- Check file permissions
- Verify MP4 is not corrupted: `ffmpeg -i file.mp4`

### Out of Memory

- Reduce image size before processing
- Process files one at a time (not in parallel)
- Use lossless mode (`--no-lossy`)

### Server Won't Start

- Check if port 3000 is in use: `lsof -i:3000`
- Kill existing process: `lsof -ti:3000 | xargs kill`
- Change port in `server/src/main.rs`

### Dependencies Won't Build

- Update Rust: `rustup update`
- Clean build: `cargo clean && cargo build`
- Check ffmpeg: `brew install ffmpeg` (macOS)

## Changelog

### v0.1.0 (2026-02-06)
- Initial workspace structure
- CLI tool with compress, convert, inspect, extract commands
- Web server with REST API
- Support for PNG, WebP, MP3, MP4 formats
- Parallel processing
- Metadata stripping
- Format conversion (PNG/JPG/WebP)
