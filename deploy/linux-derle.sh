#!/usr/bin/env bash
# Statik (musl) Linux ikilileri: glibc sürümüne bağlı değil, her Ubuntu'da çalışır.
# Kullanım: deploy/linux-derle.sh <çıktı klasörü>
# sudo gerekmez: musl-tools yoksa cargo-zigbuild ve zig ~/.divan-derleme altına kurulur.
set -euo pipefail

CIKTI=$1
ARAC="$HOME/.divan-derleme"
HEDEF=x86_64-unknown-linux-musl
ZIG=zig-linux-x86_64-0.13.0
export PATH="$HOME/.cargo/bin:$ARAC/bin:$ARAC/$ZIG:$PATH"
export CARGO_TARGET_DIR="$ARAC/target"

rustup target add "$HEDEF" >/dev/null
if command -v musl-gcc >/dev/null; then
  cargo build --release --target "$HEDEF" -p divan -p ulak
else
  mkdir -p "$ARAC"
  command -v cargo-zigbuild >/dev/null || cargo install --quiet --root "$ARAC" cargo-zigbuild
  command -v zig >/dev/null || curl -fsSL "https://ziglang.org/download/0.13.0/$ZIG.tar.xz" | tar -xJ -C "$ARAC"
  cargo zigbuild --release --target "$HEDEF" -p divan -p ulak
fi

mkdir -p "$CIKTI"
cp "$CARGO_TARGET_DIR/$HEDEF/release/divan" "$CARGO_TARGET_DIR/$HEDEF/release/ulak" "$CIKTI/"
chmod +x "$CIKTI/divan" "$CIKTI/ulak"
