# AnduinOS Packages

AnduinOS APKG package sources.

## Conflicts with Ubuntu packages

The following AnduinOS packages explicitly replace or conflict with Ubuntu's official packages:

| AnduinOS Package | Conflicts | Replaces |
|---|---|---|
| `alsa-ucm-conf-anduinos` | `alsa-ucm-conf` | `alsa-ucm-conf` |
| `firmware-sof-anduinos` | `firmware-sof-signed` | `firmware-sof-signed` |
| `gnome-shell-extension-appindicator` | `gnome-shell-extension-appindicator` | `gnome-shell-extension-appindicator` |
| `gnome-shell-extension-dash-to-panel-anduinos` | `gnome-shell-extension-dash-to-panel` | `gnome-shell-extension-dash-to-panel` |
| `gnome-shell-extension-gtk4-desktop-icons-ng` | — | `gnome-shell-extension-desktop-icons-ng` |
| `anduinos-gnome-extensions` | `gnome-shell-ubuntu-extensions` | `gnome-shell-ubuntu-extensions` |
| `plymouth-theme-spinner` | — | `plymouth` |

## Build

Each package is built via the GitLab CI pipeline (`.gitlab-ci.yml`). Packages use the `Aiursoft.Apkg.Sdk` and can be built locally with:

```
apkg publish
```
