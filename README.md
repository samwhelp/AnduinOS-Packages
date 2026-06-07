# AnduinOS Packages

[![GPL licensed](https://img.shields.io/badge/license-GPL-blue.svg)](https://github.com/AiursoftWeb/AnduinOS-Packages/blob/master/LICENSE)

AnduinOS APKG package sources.

## Package conflicts

The following AnduinOS packages explicitly replace or conflict with Ubuntu's official packages.

### 🔴 Hard Replace — Conflicts + Replaces + Provides

These packages remove the Ubuntu equivalent at install time. The user **cannot** install both.

| AnduinOS Package | Replaces (Ubuntu) | Also Provides | Notes |
|---|---|---|---|
| `anduinos-no-snapd` | `snapd` | — | APT pin (-10) + unmounts & purges /snap, /var/snap |
| `anduinos-desktop` | `ubuntu-desktop`, `yaru-theme-gnome-shell`, `update-notifier`, `update-notifier-common`, `update-manager`, `update-manager-core`, `ubuntu-release-upgrader-core`, `ubuntu-release-upgrader-gtk` | `yaru-theme-gnome-shell` | Metapackage — 8 packages blocked, depends on `anduinos-no-snapd` |
| `anduinos-session` | `ubuntu-session` | `ubuntu-session` | + postinst purges `10_ubuntu-session.gschema.override` |
| `anduinos-gnome-extensions` | `gnome-shell-ubuntu-extensions` | `gnome-shell-ubuntu-extensions` | Metapackage — AnduinOS-curated extension set |
| `anduinos-installer-config` | `ubiquity-slideshow-ubuntu` | `ubiquity-slideshow-ubuntu` | + postinst `dpkg-divert` of languagelist |
| `anduinos-fonts` | `fonts-noto-color-emoji` | `fonts-noto-color-emoji` | Ships CascadiaCode, NerdFonts, Noto Sans/Serif, TwitterColorEmoji |
| `anduinos-software-properties-common` | `software-properties-common` | `software-properties-common` | Patches `add-apt-repository` → forces `--distro=ubuntu` for PPA compat |
| `anduinos-software-properties-gtk` | `software-properties-gtk` | `software-properties-gtk` | Strips Ubuntu Pro ads from source; suppresses dep on `ubuntu-pro-client` |
| `firefox-anduinos` | `firefox` | — | Mozilla .deb, not the snap wrapper |
| `firmware-sof-anduinos` | `firmware-sof-signed` | `firmware-sof-signed` | Newer Intel SOF snapshot from `thesofproject/sof-bin` |
| `alsa-ucm-conf-anduinos` | `alsa-ucm-conf` | `alsa-ucm-conf` | Newer snapshot (`1.2.16` vs `1.2.15.3`) |
| `plymouth-anduinos` | `plymouth-theme-spinner` | `plymouth-theme-spinner` | Boot splash — derives upstream spinner into `themes/anduinos/` namespace |

#### GNOME Shell Extensions (same UUID = dpkg file conflict)

| AnduinOS Package | Replaces (Ubuntu) | Also Provides |
|---|---|---|
| `gnome-shell-extension-dash-to-panel-anduinos` | `gnome-shell-extension-dash-to-panel` | `gnome-shell-extension-dash-to-panel` |
| `gnome-shell-extension-desktop-icons-ng-anduinos` | `gnome-shell-extension-desktop-icons-ng`, `gnome-shell-extension-gtk4-desktop-icons-ng` | both |
| `gnome-shell-extension-appindicator-anduinos` | `gnome-shell-extension-appindicator` | `gnome-shell-extension-appindicator` |
| `gnome-shell-extension-gtk4-desktop-icons-ng` | `gnome-shell-extension-desktop-icons-ng`, `gnome-shell-extension-desktop-icons-ng-anduinos` | `gnome-shell-extension-desktop-icons-ng` |

---

### 🟡 Dual Recommend — user can switch back to Ubuntu version

These packages use `anduinos-X | ubuntu-X` in `Depends` or `Recommends`. The AnduinOS version is preferred (listed first), but the user can `apt install ubuntu-X` to swap.

| Preferred (AnduinOS) | Fallback (Ubuntu) | Where defined |
|---|---|---|
| `anduinos-session` | `ubuntu-session` \| `gnome-session` | `anduinos-desktop-core` → `Dependency` |
| `firmware-sof-anduinos` | `firmware-sof-signed` | `anduinos-desktop-core` → `Dependency` |
| `alsa-ucm-conf-anduinos` | `alsa-ucm-conf` | `anduinos-desktop-core` → `Dependency` |

---

### 🟢 Soft Override — file shadow, no package blocks

These packages replace Ubuntu **files** without removing the Ubuntu **package**.

| AnduinOS Package | What it overrides | Mechanism |
|---|---|---|
| `base-files` | `/etc/os-release`, `/etc/lsb-release`, `/etc/issue`, `/etc/issue.net`, `/usr/share/pixmaps/ubuntu-logo-*.png`, `/etc/legal` | File deployment (epoch `1:` outranks) |
| `anduinos-apt-config` | APT repo configuration | Pin priority 1001 for AnduinOS origin; `.sources` + `.pref` files |
| `anduinos-mimeapps` | `/usr/share/applications/gnome-mimeapps.list` | `dpkg-divert` in preinst, restored on postrm |
| `anduinos-bwrap-hack` | `/usr/bin/bwrap` → `/usr/bin/bwrap.real` + wrapper | Shim renames the real binary, wrapper swallows failures |

---

### Chains of removal

Installing `anduinos-desktop` triggers a cascade:

```
anduinos-desktop
├── Conflicts → ubuntu-desktop, ubuntu-release-upgrader*, update-notifier*, update-manager*, yaru-theme-gnome-shell
├── Depends → anduinos-desktop-core
│   ├── Conflicts → (none directly)
│   └── Depends → anduinos-session | ubuntu-session | gnome-session
│               → firmware-sof-anduinos | firmware-sof-signed
│               → alsa-ucm-conf-anduinos | alsa-ucm-conf
├── Depends → anduinos-no-snapd
│   └── Conflicts → snapd
└── Recommends → anduinos-software-properties-common
               → anduinos-software-properties-gtk (resolute only)
```

**14 Ubuntu packages** are removed or pinned out when the full AnduinOS desktop is installed.

## Build

Each package is built via the GitLab CI pipeline (`.gitlab-ci.yml`). Packages use the `Aiursoft.Apkg.Sdk` and can be built locally with:

```
apkg publish
```

## Monthly Update Manual

All external sources must be checked **at least once per month** to keep packages from falling behind upstream. This section is the step-by-step checklist.

---

### A. Quick Checklist (5 min triage)

Run through this table each month. If anything has changed upstream, follow the detailed steps in the matching section below.

| # | What | Where to check | Update action |
|---|---|---|---|
| 1 | **Fluent GTK theme** | `anduinos-fluent-gtk-theme/download.sh:5` (commit) + upstream [releases] | Update commit → section B |
| 2 | **Fluent icon theme** | `anduinos-fluent-icon-theme/download.sh:5` (commit) + upstream [releases] | Update commit → section B |
| 3 | **ALSA UCM Conf** | `alsa-ucm-conf-anduinos/download.sh:5` (commit) + upstream [repo] | Update commit → section B |
| 4 | **SOF firmware** | `firmware-sof-anduinos/download.sh:5` (`SOF_VERSION`) + upstream [releases] | Update version → section C |
| 5 | **GNOME Shell version map** | `lib/gnome-versions.sh:3-7` — compare with Ubuntu's `gnome-shell` package for each supported suite | Update map → section D |
| 6 | **Fluent upstream versions** | [Fluent-gtk-theme] and [Fluent-icon-theme] upstream — compare with `*.aosproj` PackageVersion | Update version → section B |
| 7 | **GNOME Shell extensions** | Run a CI build — the resolver fetches the latest compatible version dynamically | Update version → section D |

[releases]: https://github.com/vinceliuice/Fluent-gtk-theme
[repo]: https://github.com/alsa-project/alsa-ucm-conf

---

### B. Git-Pinned Packages (Fluent GTK, Fluent Icon, ALSA UCM Conf)

Three packages clone a git repo and pin to a specific commit hash. Both the **commit hash** and the **`.aosproj` PackageVersion** must be updated together.

#### B.1 Check for updates

```bash
# Fluent GTK theme (upstream: vinceliuice/Fluent-gtk-theme)
git ls-remote https://github.com/vinceliuice/Fluent-gtk-theme.git HEAD

# Fluent icon theme (upstream: vinceliuice/Fluent-icon-theme)
git ls-remote https://github.com/vinceliuice/Fluent-icon-theme.git HEAD

# ALSA UCM Conf (upstream: alsa-project/alsa-ucm-conf)
git ls-remote https://github.com/alsa-project/alsa-ucm-conf.git HEAD
```

Compare the returned HEAD hash against the pinned commit in each `download.sh`. If different, an update is available.

For **Fluent** packages, also check the upstream tag/release to determine the new version number. For **ALSA UCM Conf**, check the upstream `configure.ac` for the version.

#### B.2 Apply the update

For each outdated package, update **two files**:

1. **`download.sh`** — update the `*_COMMIT` variable to the new HEAD hash
2. **`*.aosproj`** — bump `<PackageVersion>` to match the new upstream version, reset the Debian revision suffix (e.g. `2.0.2` → `2.0.3-1`)

Example diff for Fluent GTK theme:

```diff
# download.sh
-FLUENT_GTK_COMMIT="9fc5291"
+FLUENT_GTK_COMMIT="a1b2c3d"

# anduinos-fluent-gtk-theme.aosproj
-<PackageVersion>2.0.2</PackageVersion>
+<PackageVersion>2.0.3-1</PackageVersion>
```

#### B.3 Rebuild and verify

```bash
cd <package-dir>
apkg publish
```

---

### C. Firmware SOF (Version-Pinned Tarball)

`firmware-sof-anduinos` now derives from Ubuntu's `firmware-sof-signed` package at build time, then replaces the unpacked SOF payload with a newer Intel `sof-bin` release tarball. This keeps file ownership, upgrades, and removals under dpkg instead of a post-install `rsync`.

#### C.1 Check for updates

Visit [sof-bin releases](https://github.com/thesofproject/sof-bin/releases) and compare the latest tag against `SOF_VERSION` in `firmware-sof-anduinos/download.sh`.

#### C.2 Apply the update

Update **two files**:

```diff
# download.sh
-SOF_VERSION="2025.12"
+SOF_VERSION="2026.03"   # update to new release tag

# firmware-sof-anduinos.aosproj
-<PackageVersion>2025.12+ubuntu$(UpstreamVersion)-1</PackageVersion>
+<PackageVersion>2026.03+ubuntu$(UpstreamVersion)-1</PackageVersion>   # update the leading SOF version, reset suffix if packaging changed
```

The downloaded Intel tarball and extracted cache under `deploy/` are **not** committed — the CI regenerates them at build time via `download.sh`.

#### C.3 Rebuild and verify

```bash
cd firmware-sof-anduinos
apkg publish
```

---

### D. GNOME Shell Extensions (19 packages)

These are resolved **dynamically at build time**: the resolver (`lib/resolve-gnome-ext.py`) queries `extensions.gnome.org` for the best compatible version for each target GNOME Shell version. This means extension code is always up-to-date on every build — no monthly check needed for the extension code itself.

However, two things still need attention:

#### D.1 GNOME Shell version map (`lib/gnome-versions.sh`)

This file maps Ubuntu suite codenames to GNOME Shell major versions. It must be updated whenever:

- A new Ubuntu release ships (new suite codename + new GNOME Shell version)
- An existing Ubuntu point-release updates its GNOME Shell version (rare)

To check:

```bash
# For each supported suite, check what GNOME Shell version Ubuntu ships:
apt-cache policy gnome-shell  # (run on each supported Ubuntu release)
```

If a mismatch is found, update `lib/gnome-versions.sh`:

```bash
declare -A GNOME_TARGETS=(
    [noble]=46      # Ubuntu 24.04 LTS
    [questing]=49   # Ubuntu 25.10
    [resolute]=50   # Ubuntu 26.04 LTS
    # ^ update or add entries as needed
)
```

Then **CI rebuilds all 19 extension packages automatically** — the new GNOME version will be picked up by the resolver on the next build.

#### D.2 Extension `.aosproj` version numbers

Each extension's `.aosproj` has a `<PackageVersion>` like `69.2+$(SuiteShortName)8`. The prefix (e.g. `69.2`) represents the upstream extension version at the time of packaging. While the resolver always fetches the latest code, it's good practice to keep the version prefix in sync with the actual upstream version.

To audit:

```bash
# For one extension, check the version being resolved:
python3 lib/resolve-gnome-ext.py "arcmenu@arcmenu.com" --target 50 --download --out /tmp/test-ext
# The downloaded metadata.json will contain the resolved version.
# Compare against the prefix in the .aosproj PackageVersion.
```

If the upstream extension version has bumped significantly, update the prefix in the `.aosproj`:

```diff
-<PackageVersion>69.2+$(SuiteShortName)8</PackageVersion>
+<PackageVersion>72.0+$(SuiteShortName)1</PackageVersion>
```

Note: the `+$(SuiteShortName)8` suffix is a packaging revision — increment it when the packaging changes but the upstream version stays the same.

#### D.3 Special-cased extension: desktop-icons-ng-anduinos

`gnome-shell-extension-desktop-icons-ng-anduinos` explicitly conflicts with Ubuntu's `gnome-shell-extension-desktop-icons-ng` (same UUID `ding@rastersoft.com`). It also carries a custom `metadata.patch` in its deploy directories. When the upstream DING extension releases a new version, verify the patch still applies cleanly.

---

### E. Upstream-Derived Packages

Six packages derive from upstream `.deb` packages at build time via `UpstreamUrl`:

| Package | Upstream source | Repository |
|---|---|---|
| `base-files` | `base-files` | Ubuntu mirror |
| `firmware-sof-anduinos` | `firmware-sof-signed` | Ubuntu mirror |
| `plymouth-anduinos` | `plymouth-theme-spinner` | Ubuntu mirror |
| `anduinos-software-properties-common` | `software-properties-common` | Ubuntu mirror |
| `anduinos-software-properties-gtk` | `software-properties-gtk` | Ubuntu mirror |
| `firefox-anduinos` | `firefox` | Mozilla APT (`packages.mozilla.org`) |

These are rebuilt by CI and pull the latest upstream source at build time, so the **upstream base** stays up-to-date. `firmware-sof-anduinos` still needs the separate Intel release check from section C.

**Monthly check**: confirm the mirrors (`https://mirror.aiursoft.com/ubuntu` and Mozilla APT) are syncing correctly. No code changes needed unless the upstream changes the package name or the mirror URL changes.

---

### F. Internal Infrastructure (As-Needed)

These URLs are not on a monthly schedule, but should be reviewed whenever infrastructure changes:

| Reference | File | Purpose |
|---|---|---|
| `https://apkg-dev.aiursoft.com` | `.gitlab-ci.yml:7` | APKG build server |
| `https://apkg-dev.aiursoft.com/artifacts/certs/anduinos` | `anduinos-archive-keyring.aosproj:17` | GPG signing key |
| `https://apkg-dev.aiursoft.com/artifacts/anduinos/` | `anduinos-apt-config/deploy/**/*.sources` | APT repo (shipped to users!) |
| `https://nuget.aiursoft.com/v3/index.json` | `nuget.config:5` | NuGet source for build tools |
| `https://dl.flathub.org/repo/flathub.flatpakrepo` | `anduinos-appstore/scripts/postinst.sh:4` | Flathub remote |
| `https://mirror.aiursoft.com/ubuntu` | Multiple `.aosproj` files | Ubuntu mirror |

---

### G. Monthly Update Order

When doing a full monthly triage, follow this order — earlier packages are dependencies of later ones:

1. **Fluent icon theme** + **Fluent GTK theme** (no deps)
2. **ALSA UCM Conf** + **SOF firmware** (alsa-ucm-conf depends on nothing; SOF recommends alsa-ucm-conf but doesn't build-depend on it)
3. **GNOME Shell extension** version audits (no cross-deps)
4. **GNOME version map** update (triggers extension rebuilds)
5. **Ubuntu-derived packages** (no cross-deps; build picks up latest)
6. **Meta-packages** — rebuild last (`anduinos-desktop`, `anduinos-desktop-core`, `anduinos-gnome-extensions`, `anduinos-theme`)

For each updated package, push to `master` — CI runs `apkg publish && apkg push` automatically.
