class Vladini < Formula
  desc "Compress, convert and strip metadata of images, audio and video"
  homepage "https://github.com/bulbazavr1024/vladini"
  url "https://github.com/bulbazavr1024/vladini/releases/download/v0.2.0/vladini-macos-arm64.tar.gz"
  version "0.2.0"
  sha256 "183a74d3c6bd73547fefb3b78ffa3f1e5c890bed895e7d70303503df0b616336"

  depends_on arch: :arm64
  depends_on :macos
  depends_on "ffmpeg"

  def install
    bin.install "vladini"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/vladini --version")
  end
end
