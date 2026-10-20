EAPI=8

inherit cargo

DESCRIPTION="Cockpit-grade system fetch with a configurator and distro logos"
HOMEPAGE="https://github.com/mohamedsalahi/tachyon-fetch"
SRC_URI="
	https://github.com/mohamedsalahi/tachyon-fetch/archive/refs/tags/v${PV}.tar.gz -> ${P}.tar.gz
	$(cargo_crate_uris)
"
S="${WORKDIR}/${PN}-${PV}"

LICENSE="MIT"
SLOT="0"
KEYWORDS="~amd64"
IUSE=""

DEPEND=""
RDEPEND="media-libs/fontconfig"
BDEPEND="virtual/rust"

# Generated from Cargo.lock with app-portage/cargo-ebuild; fill before
# publishing the overlay:
# CRATES="
# "
