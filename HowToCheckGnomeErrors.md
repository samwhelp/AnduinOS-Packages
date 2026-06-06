# How to Diagnose GNOME Shell Extension Errors

When GNOME behaves abnormally (extensions crashing, UI glitches, shell lag), here is a systematic approach to find the root causes.

## Step 1: Gather context

```bash
gnome-shell --version
gnome-extensions list
```

Know what GNOME version you are on and which extensions are installed. GNOME Shell and its extensions are tightly version-coupled — the #1 cause of extension failures is a version mismatch after a Shell upgrade.

## Step 2: Scan all log sources in parallel

Don't just look at one log. Different errors land in different places:

```bash
# User journal (where GNOME Shell runs) — errors only, last hour
journalctl --user -p 3 --since "1 hour ago" -o cat --no-pager

# System journal for your UID
journalctl _UID=$(id -u) -p 3 --since "1 hour ago" -o cat --no-pager

# X/Wayland session log
cat ~/.xsession-errors | tail -80

# Syslog filtered for GNOME-related daemons
grep -iE "gnome|gjs|gdm|mutter|shell" /var/log/syslog | tail -80

# Kernel-level issues (GPU/driver failures show up here)
dmesg --level=emerg,alert,crit,err | tail -30
```

## Step 3: Deep-grep the full user journal

GNOME Shell logs most extension problems at INFO level, not ERROR level — so `-p 3` will miss them. You must grep the raw stream:

```bash
# Cast a wide net across ALL of today's user journal
journalctl --user --since "today" --no-pager -o cat \
  | grep -iE "gnome-shell|gjs|extension.*error|extension.*fail|extension.*crash|mutter"
```

Then narrow to specific categories:

```bash
# JS-level errors (the most common extension failure type)
journalctl --user --since "today" --no-pager -o cat \
  | grep -iE "gjs|JS ERROR|extension" | tail -60

# All errors/exceptions/failures, excluding known-noisy patterns
journalctl --user --since "today" --no-pager -o cat \
  | grep -iE "error|crash|fail|critical|exception|segfault|traceback" \
  | grep -vE "offending signal|needs an allocation" | tail -60
```

## Step 4: Check crash dumps

```bash
ls -lt /var/crash/
ls -lt ~/.cache/gnome-shell/
```

A `.crash` file in `/var/crash/` for a `gnome-shell-extension-*` entry is a smoking gun — that extension crashed at least once.

Read the crash file for the stack trace:

```bash
cat /var/crash/gnome-shell-extension-<name>.0.crash | head -100
```

## Step 5: Check individual extension status

```bash
gnome-extensions info <extension-uuid>
```

`State: ACTIVE` means it loaded successfully. `State: ERROR` or `State: OUT-OF-DATE` confirms the problem.

Also check the extension files themselves for missing components:

```bash
# Does it have the required files?
ls -la /usr/share/gnome-shell/extensions/<uuid>/
ls -la ~/.local/share/gnome-shell/extensions/<uuid>/

# If it's a GJS script, does it have execute permission?
ls -la /usr/share/gnome-shell/extensions/<uuid>/app/*.js
```

## Step 6: Check for update artifacts

GNOME Shell upgrades can leave extensions in a broken state. Specific things to look for:

```bash
# Extensions installed but missing metadata.json
find /usr/share/gnome-shell/extensions/ -maxdepth 1 -type d \
  ! -name "extensions" \
  ! -exec test -f "{}/metadata.json" \; \
  -print 2>/dev/null

# Extensions left over from a prior Shell version (check API compatibility)
gnome-extensions list --disabled
```

## Common failure signatures

| Symptom in log | What it means |
|---|---|
| `Missing metadata.json` | Extension directory exists but is incomplete (partial install/uninstall) |
| `Permission denied` on `.js` file | Script missing `+x` bit; extension forks a subprocess that can't start |
| `ImportError: Unable to load file` | Resource file the extension depends on was moved/removed |
| `Can't update stage views actor ... needs an allocation` | Widget layout loop — often a dash/panel extension fighting Shell's native layout |
| `Name "org.gnome.Shell.Extensions" does not exist` | Shell D-Bus service not yet available (transient during startup) |
| `.crash` in `/var/crash/` | Hard crash — check the stack for the specific GJS function that blew up |

## Quick fix checklist

1. Fix permissions: `sudo chmod +x /path/to/broken/script.js`
2. Purge broken directories: `sudo rm -rf /usr/share/gnome-shell/extensions/<broken-uuid>`
3. Restart Shell: `Alt+F2` → type `r` → `Enter` (or log out and back in)
4. If an extension still fails after a Shell upgrade, check upstream for a compatible version

## One-liner diagnostic

Run this first when someone reports "GNOME is broken":

```bash
journalctl --user --since "today" --no-pager -o cat | grep -iE "error|fail|crash|permission denied|missing|import.*error|can't|unhandled" | grep -vE "sudo.*auth|frame latency|offending signal|needs an allocation|DEPRECATED_ENDPOINT"
```

The `grep -vE` part filters out known noise (Chrome frame latency, sudo auth failures, clutter allocation chatter, etc.) that is almost never the actual problem.
