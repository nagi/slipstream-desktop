# Installing Slipstream on Omarchy: sticking points

Notes from installing Slipstream 0.13.0 from a clone (branch `nagi`) on Omarchy 4.0.4 (Arch, kernel
7.2.5-3-omarchy, Limine + LUKS full-disk encryption, SDDM), 2026-10-21. Written up for discussion
with Pete upstream. The build and `install-session` themselves went fine; nearly all the friction
is in getting *into* the session.

## 1. You never get to choose Slipstream

Omarchy's boot is: Limine → LUKS passphrase prompt → straight into Hyprland. There is no point at
which a session can be picked, and logging out doesn't offer one either. Four things combine:

- **SDDM autologins.** `/etc/sddm.conf.d/autologin.conf` (written by the Omarchy installer, owned
  by no package):

  ```ini
  [Autologin]
  User=nagi
  Session=omarchy.desktop
  ```

  Omarchy does this deliberately: the LUKS passphrase *is* the login, so a second password prompt
  would be redundant.

- **The Omarchy SDDM theme has no session picker at all.** `/usr/share/sddm/themes/omarchy/Main.qml`
  picks the session itself, as the first entry whose name contains `uwsm`:

  ```qml
  property int sessionIndex: {
    for (var i = 0; i < sessionModel.rowCount(); i++) {
      var name = (sessionModel.data(sessionModel.index(i, 0), Qt.DisplayRole) || "").toString()
      if (name.indexOf("uwsm") !== -1)
        return i
    }
    return sessionModel.lastIndex
  }
  ```

  So even with autologin turned off, the login screen is just a password box that always logs
  into `Omarchy (Hyprland uwsm)`. Slipstream's `slipstream.desktop` is installed and in
  `sessionModel`, but nothing can select it.

- **SDDM only autologins once.** Its default is `Relogin=false`
  (`/usr/lib/sddm/sddm.conf.d/default.conf`), so autologin fires when SDDM starts, i.e. at boot.
  After a logout you get the greeter, and therefore the point above.

- **Logout goes back round the same loop.** `omarchy-system-logout` runs `uwsm stop` → SDDM greeter
  → the picker-less theme → Omarchy again.

**What the installer says today** (`scripts/install-session --check`, text at
`scripts/install-session:553`):

> This machine logs in automatically: nagi, straight into omarchy.desktop, so no login screen
> appears to choose Slipstream on. Set Session=slipstream.desktop in sddm's [Autologin] section to
> log straight into Slipstream instead, or remove the [Autologin] section to get the list back.

The installer does detect Omarchy ("Slipstream on Omarchy"), and the first half of the advice is
right. The second half is wrong here: removing `[Autologin]` doesn't "get the list back", because
the Omarchy theme has no list. And setting `Session=slipstream.desktop` by hand makes Slipstream
the only desktop you can reach without editing a root-owned file again, from a TTY if
Slipstream won't start.

### What I did to work around it

A switch in both directions, with no password, set up by one command (run from Omarchy, after
`scripts/update-session --no-extras` and `scripts/install-session`):

```sh
extras/omarchy-switch/install              # or --uninstall to take it all out again
```

| File | What it is |
|---|---|
| `/usr/local/bin/slipstream-set-autologin` (root, 755) | Takes exactly `omarchy` or `slipstream` and rewrites the `Session=` line of `autologin.conf` (temp file + `mv`). |
| `/etc/sudoers.d/slipstream-switch` (440) | `NOPASSWD` for exactly those two invocations of that helper, nothing else. Written for whoever runs the installer, and checked with `visudo`. |
| `/etc/sddm.conf.d/zz-slipstream-switch.conf` | `[Autologin] Relogin=true`, so a logout autologins again into whatever `Session=` now says. Also `[Theme] Current=omarchy-slipstream`. Named `zz-` so it's read after `autologin.conf`, which on a fresh Omarchy also sets a theme. |
| `/usr/share/sddm/themes/omarchy-slipstream` | Omarchy's login theme with F2 to cycle sessions (`extras/omarchy-switch/login-theme`). Only seen if automatic login is ever turned off. |
| `~/.local/bin/desktop-switch [omarchy\|slipstream]` | Calls the helper, then ends the desktop the way its own logout does. In Omarchy, `omarchy-system-logout` (closes apps gracefully, then `uwsm stop`). In Slipstream, SIGTERM to the compositor, which shuts down cleanly. On a text console, `systemctl restart sddm` (also allowed by the sudoers rule), since there's no desktop to end. Never `loginctl terminate-session`: that kills SDDM's helper too, whose exit code 1 SDDM reads as a wrong password, so it neither logs back in nor shows a login screen. |
| Ctrl+Alt+Delete at the end of `~/.config/hypr/bindings.lua` | Omarchy → Slipstream. Unbinds Omarchy's own Ctrl+Alt+Delete (close all windows) first; the logout closes them anyway. Slipstream → Omarchy is Ctrl+Alt+Delete there too, then O on the way out. |
| `~/.local/share/applications/desktop-switch-omarchy.desktop` | "Switch to Omarchy" in Slipstream's app explorer (`OnlyShowIn=Slipstream`), for the way back. |

The choice persists, so after a reboot you land in whichever desktop you were last in. No reboot
is needed for the switch itself: after a logout SDDM makes a new display, and that reloads any
config file that changed (`Seat::createDisplay` → `mainConfig.load()`, SDDM 0.21).

`Relogin=true` would restart a Slipstream that dies at startup over and over. So
`session/slipstream-session` times the session: if it ends within 15 seconds and the switch is
installed, it points autologin back at Omarchy first. It then exits 0 whatever Slipstream's
status, for the same exit-code-1 reason as above (logged in `~/.local/state/slipstream/
slipstream.log`). Escape hatch if all else fails: Ctrl+Alt+F3, log in, `desktop-switch omarchy` (restarts SDDM).

One side effect: Slipstream's own Log out now logs straight back in to Slipstream. Use
`desktop-switch` to leave it.

### Suggestions for upstream

1. **Make the installer's advice Omarchy-aware.** When the SDDM theme is `omarchy`, say the theme
   has no session list. Offer the switch mechanism above (or something like it) instead of
   suggesting `[Autologin]` be removed.
2. **Ship a switch command.** Something like `slipstream-switch` that `install-session` installs
   (and `--uninstall` removes) when it finds SDDM autologin. It would be the helper + sudoers rule
   + `Relogin=true`, plus a "Switch to <other desktop>" entry. This is general, not just Omarchy:
   any autologin setup has the same problem.
3. **Let Slipstream bind a key to a command.** There's no way to put "switch back to Omarchy" on a
   key inside Slipstream, so the way back is the app explorer. A `[keys]` table of user commands
   would cover this and a lot else.
4. **Upstream to Omarchy.** The theme could honour `RememberLastSession` / the
   last-used session instead of hard-coding "uwsm", or offer a key to cycle sessions. Then other
   desktops could coexist on Omarchy without any of the above.
5. **A Limine entry per desktop** is the other approach: pick the desktop at boot (e.g. a kernel
   command-line flag that a tiny SDDM pre-start hook turns into `Session=`). It's more invasive,
   since Limine's config is Omarchy's (`/etc/default/limine`, regenerated by `limine-update`), so
   I didn't do it.

## 2. Building from source on Arch is undocumented

`guide/install.md` gives build dependencies for Ubuntu and Fedora only. On Omarchy:

- **`mold` is not installed**, but `.cargo/config.toml` hard-wires `-fuse-ld=mold`, so the build
  fails at the first build script. Needed: `sudo pacman -S mold`. Arguably the repo config
  shouldn't force a linker that isn't listed as a requirement, or `update-session` should fall
  back when it's absent.
- **`rustup` is not installed**; Omarchy ships Arch's `rust` package. The guide says "rustup fetches
  the Rust version Slipstream needs". That only worked here because Arch's rust happened to be
  1.98.1, the exact pinned `rust-toolchain.toml` channel. When Arch moves on, cargo will ignore the
  pin and build with whatever Arch has, or the build will need `rustup`.
- Everything else was already present (libxkbcommon, wayland, libinput, mesa, seatd/libseat,
  libdrm, pixman, libdisplay-info, gtk4, clang, pkg-config). After that, `scripts/update-session`
  built and installed cleanly in 1m12s with no warnings.

Suggested Arch line for the guide:

```sh
sudo pacman -S --needed base-devel clang mold pkgconf rustup libxkbcommon wayland systemd-libs \
    libinput mesa seatd libdrm pixman libdisplay-info gtk4
```

## 3. Missing runtime package

- `xdg-desktop-portal-wlr` isn't installed on Omarchy (Hyprland uses
  `xdg-desktop-portal-hyprland`), so screen sharing doesn't work until it's added. The installer
  reports this correctly and gives the right `pacman` command.

## 4. Omarchy's user services start inside Slipstream too

Omarchy enables several systemd user services `WantedBy=graphical-session.target`, which Slipstream
binds to, so they start in a Slipstream session as well:

- `omarchy-fcitx5` (input method): probably wanted.
- `omarchy-crash-watch`, `omarchy-migrate-notify`: harmless, though they may send notifications
  that assume Omarchy.
- `omarchy-sleep-lock` runs `omarchy-system-sleep-monitor`, which locks with Omarchy's locker
  before suspend. It has `ConditionEnvironment=OMARCHY_PATH`, which is only in the user manager's
  environment because uwsm imports it. So after a fresh boot into Slipstream it should stay off.
  Worth checking whether it starts after a switch from Omarchy without a reboot, if `OMARCHY_PATH`
  is left over in the user manager. If it does, two lockers would race at suspend.
- `omarchy-recover-internal-monitor` (`WantedBy=graphical-session-pre.target`) will also run.

To watch for, not confirmed as problems yet.

## 5. Small things

- **`slipstream --help` starts the compositor.** Unknown arguments are ignored, so `--help` opened
  a nested Slipstream window instead of printing usage. A `--help` that prints the flags
  (`--session`, `--tty`, `--version`, …) would help.
- **`install-session --check` in a clone says "programs: none in this folder; scripts/update-session
  builds them into your home"**, even after `update-session` has put them in `~/.local/bin`. It
  could say they're already there.
- **`OnlyShowIn=Slipstream` fails `desktop-file-validate`** ("unregistered value … should start
  with X-"). Slipstream's own catalog matches it fine, but other tools may complain. Worth knowing
  if Slipstream documents `XDG_CURRENT_DESKTOP=Slipstream` for app authors.
