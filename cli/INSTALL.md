# Installation Guide

> **Note**: This is the CLI tool from the Vladini workspace. For workspace documentation, see `../README.md`.

## Quick Install (Recommended) ✅

macOS on Apple Silicon, prebuilt binary via Homebrew:

```bash
brew tap bulbazavr1024/vladini https://github.com/bulbazavr1024/vladini
brew install vladini          # also installs ffmpeg
brew upgrade vladini          # update to the latest release
```

## Install From Source

Install the binary globally to `~/.cargo/bin/`:

**From workspace root:**
```bash
cd /path/to/vladini
cargo install --path cli
```

**Or from CLI directory:**
```bash
cd /path/to/vladini/cli
cargo install --path .
```

You can now use it from anywhere:

```bash
vladini compress photo.png -q 80
vladini convert image.png --to jpg
vladini inspect video.mp4
vladini extract video.mp4 ./frames/
```

## Installation Methods

### Method 1: Cargo Install ⭐ (Recommended)

This installs the binary to `~/.cargo/bin/` which is already in your PATH.

**From workspace root:**
```bash
cd /path/to/vladini
cargo install --path cli
```

**From CLI directory:**
```bash
cd /path/to/vladini/cli
cargo install --path .
```

**To update after making changes:**
```bash
cargo install --path cli --force  # From workspace root
# OR
cargo install --path . --force     # From cli directory
```

**To uninstall:**
```bash
cargo uninstall vladini
```

**Benefits:**
- ✅ Available system-wide
- ✅ Easy to update
- ✅ Clean uninstall
- ✅ No sudo required

### Method 2: System-wide Install

Copy to `/usr/local/bin/` (requires sudo):

```bash
# From workspace root
cargo build --release --bin vladini
sudo cp target/release/vladini /usr/local/bin/

# OR from cli directory
cd cli
cargo build --release
sudo cp target/release/vladini /usr/local/bin/
```

**To update:**
```bash
cargo build --release --bin vladini
sudo cp target/release/vladini /usr/local/bin/
```

**To uninstall:**
```bash
sudo rm /usr/local/bin/vladini
```

### Method 3: Shell Alias

Add to `~/.zshrc` (or `~/.bashrc` for bash):

```bash
# Update path to match your workspace location
echo 'alias vladini="$HOME/path/to/vladini/target/release/vladini"' >> ~/.zshrc
source ~/.zshrc
```

**To update:** Just rebuild with `cargo build --release --bin vladini` from workspace root

**To uninstall:** Remove the line from `~/.zshrc`

### Method 4: Symlink

Create a symbolic link (good for development):

```bash
ln -s ~/path/to/project/target/release/vladini /usr/local/bin/vladini
```

**Benefits**: Updates automatically when you rebuild

**To uninstall:**
```bash
rm /usr/local/bin/vladini
```

## Verify Installation

```bash
which vladini
# Output: /Users/username/.cargo/bin/vladini

vladini --version
# Output: vladini 0.1.0

vladini --help
# Shows all commands
```

## Usage Examples

Now you can use it from anywhere:

```bash
# Compress
vladini compress ~/Pictures/photo.png -q 80

# Convert
vladini convert ~/Pictures/photo.png --to webp

# Convert audio/video (requires ffmpeg)
vladini convert ~/Videos/clip.webm --to mp3

# Inspect
vladini inspect ~/Videos/video.mp4

# Extract
vladini extract ~/Videos/movie.mp4 ~/Desktop/frames/

# Resize and compress
vladini compress ~/Pictures/photo.png --width 1200 -q 85

# Batch process
cd ~/Pictures
vladini compress . -r -q 85
```

## System Requirements

### Required

- **Rust** 1.70+ ([install from rustup.rs](https://rustup.rs))
- **Cargo** (comes with Rust)

### Optional (for MP4 processing and audio/video conversion)

- **ffmpeg**
  ```bash
  # macOS
  brew install ffmpeg

  # Linux (Ubuntu/Debian)
  sudo apt install ffmpeg

  # Linux (Fedora)
  sudo dnf install ffmpeg

  # Check installation
  ffmpeg -version
  ```

## Troubleshooting

### Command not found

If `vladini` is not found, check your PATH:

```bash
echo $PATH | grep -o "[^:]*cargo[^:]*"
```

Should show: `/Users/username/.cargo/bin`

If not, add to `~/.zshrc` or `~/.bashrc`:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
source ~/.zshrc  # or source ~/.bashrc
```

### Permission denied

If you get permission errors:

```bash
chmod +x ~/.cargo/bin/vladini
```

### Old version running

After rebuilding, make sure the new version is installed:

```bash
cargo install --path . --force
vladini --version
```

### ffmpeg not found

For MP4 processing and audio/video conversion, ffmpeg must be installed:

```bash
# Check if installed
ffmpeg -version

# If not, install
brew install ffmpeg  # macOS
apt install ffmpeg   # Linux
```

## Development Workflow

### For Active Development

Use symlink or alias methods so changes apply immediately after `cargo build --release`.

**Recommended:**
```bash
# Create symlink
ln -s $(pwd)/target/release/vladini /usr/local/bin/vladini

# Now rebuild updates automatically
cargo build --release
```

### For Stable Use

Use `cargo install` and update with `--force` when needed:

```bash
# After making changes
cargo build --release
cargo install --path . --force
```

## Updating

### Update from Git

```bash
cd /path/to/project
git pull
cargo install --path . --force
```

### Update Dependencies

```bash
cargo update
cargo build --release
cargo install --path . --force
```

## Uninstalling

### If installed via cargo

```bash
cargo uninstall vladini
```

### If installed to /usr/local/bin

```bash
sudo rm /usr/local/bin/vladini
```

### If using symlink

```bash
rm /usr/local/bin/vladini
```

### If using alias

Remove the alias line from `~/.zshrc` or `~/.bashrc`

## Quick Reference

```bash
# Install
cargo install --path .

# Update
cargo install --path . --force

# Uninstall
cargo uninstall vladini

# Verify
which vladini
vladini --version

# Use
vladini compress photo.png -q 80
vladini convert image.png --to jpg
vladini inspect file.mp4
vladini extract video.mp4 ./frames/
```

---

✅ **You're all set!** Use `vladini` from anywhere in your terminal.

For usage examples, see [README.md](./README.md).
