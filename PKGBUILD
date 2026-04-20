# Maintainer: fibsussy <fibsussy@tuta.io>
pkgname=crosshair-maker
pkgver=0.1.0
pkgrel=1
pkgdesc="Crosshair overlay creator with SVG rendering and preview"
arch=('x86_64' 'aarch64')
url="https://github.com/fibsussy/crosshair-maker"
license=('MIT')
depends=('libx11' 'libxcb' 'wayland' 'libxkbcommon' 'vulkan-icd-loader')
makedepends=('rust' 'cargo')
options=('!debug')

source=()
sha256sums=()

build() {
    cd "$startdir"
    cargo build --release --locked
}

package() {
    cd "$startdir"
    install -Dm755 "target/release/crosshair-maker" "$pkgdir/usr/bin/crosshair-maker"
    install -Dm644 "LICENSE" "$pkgdir/usr/share/licenses/crosshair-maker/LICENSE"
}
