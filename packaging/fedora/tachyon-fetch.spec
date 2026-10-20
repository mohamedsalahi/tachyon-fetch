Name:           tachyon-fetch
Version:        0.7.0
Release:        1%{?dist}
Summary:        Cockpit-grade system fetch with a configurator and distro logos
License:        MIT
URL:            https://github.com/mohamedsalahi/tachyon-fetch
Source0:        %{url}/archive/refs/tags/v%{version}.tar.gz#/%{name}-%{version}.tar.gz
BuildRequires:  rust-packaging
BuildRequires:  gcc
Requires:       fontconfig

%description
A single-process system fetch that reads /proc, /sys, DMI and hwmon,
renders one composed frame, and ships a built-in configurator. ASCII
distro logos, a system-font text logo and kitty graphics artwork are
supported.

%prep
%autosetup -n %{name}-%{version}

%generate_buildrequires
%cargo_generate_buildrequires

%build
%cargo_build

%install
%cargo_install

%files
%{_bindir}/tachyon
%license LICENSE
%license logos/LICENSE-fastfetch

%changelog
* Tue Oct 20 2026 Mohamed Salahi <264395179+mohamedsalahi@users.noreply.github.com> - 0.7.0-1
- Initial package
