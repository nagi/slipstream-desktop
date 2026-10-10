# Every key

Super+K shows this list inside Slipstream, whenever you forget one. Press Enter there for the tour: sixteen short lessons for anyone new to tiling, each one drawn happening on a miniature of the desktop.

[← back to the README](../README.md)

| Keys | What they do |
|---|---|
| Super, tapped on its own | App explorer |
| Super+Return, Super+Shift+F, Super+I | Terminal, files, Settings |
| Super+Shift+B, Super+Shift+Return | A new window of your default browser |
| Alt+Tab, Alt+Shift+Tab | Switch windows, dealt as a deck of glass |
| Super+W, Alt+F4 | Close the window |
| Super+arrows | Move focus. Up from the top row goes into a docked window; from a floating or docked window they lead back to the tiles |
| Super+Shift+arrows | Move the window: it swaps places with the one that way, the two passing through each other. Under gravity, moving into the centre takes it. A floating window moves a step |
| Super+- and =, with Shift for height | Resize, tiled or floating; under gravity, lighter and heavier |
| Super+R, Super+Shift+R | Turn the whole layout a quarter, clockwise or back |
| Super+F | Fill the tiling area, and back, in tiling or gravity |
| Super+M, Super+Shift+M | Minimise to a stream at the edge, bring back |
| Super+O | Dock the window to the bar, in front of every workspace; again, its next size of three |
| Super+Shift+O | Bring the docked window back down as a standard window |
| Super+H | Hide every window on the workspace; again, bring them back |
| Super+T, Super+Alt+V | Float the window or tile it; switch between floating and tiled windows |
| Super+D | Show the desktop, and back |
| Super+Alt+Tab | Bullet time |
| Super+G | Tiling or gravity; keep Super held to choose the arrangement |
| Super+1–0, Super+Tab, Super+Shift+Tab, Super+Ctrl+← → | Go to a workspace; the next or previous one |
| Super+Shift+1–0, Super+Ctrl+Shift+← → | Move the window to a workspace |
| Super+P, Super+Shift+P | Next screen, move the window there |
| Super+Ctrl+P | Swap workspaces with the next screen, windows and all |
| Super+A, Super+N | Quick settings, notifications |
| Super+C, Super+X, Super+V | Copy, cut and paste in any app: a terminal, Emacs and gVim are each sent their own keys for it |
| Super+Ctrl+V | Clipboard history |
| Print, Super+Shift+S | Screenshot of the screen; snip a region or a window |
| Super+Ctrl+L | Lock |
| Super+Shift+Esc | Take the mouse and keys back from a game or virtual machine holding them |
| Caps Lock | Tap: Caps Lock. Hold: Awake, the screen stays on and doesn't lock; hold again or click AWAKE on the bar to end it |
| Super+Esc, Ctrl+Alt+Del | The way out: lock, log out, restart, shut down |
| Super+K | Every key, and Enter there starts the tour |

Where Omarchy has the same action, Slipstream uses Omarchy's key.

The volume, mute, brightness and media keys work as marked, with any player that speaks MPRIS, and stay the desktop's even while a game holds every key. Ctrl+Alt+F1–F12 switch virtual terminals as usual. Keys that input methods, screen readers and games rely on are left alone.

## Inside panels and bullet time

In any panel or card, Esc backs out one level, Tab and the arrows move, Enter presses, and Home, End, PgUp and PgDn jump.

| In bullet time | What they do |
|---|---|
| Arrows, Tab | Choose a window; every window in turn |
| The letter on a window | Jump to it |
| 1–0 | Look at a workspace |
| Enter | Go |
| Shift+1–0, Shift+← → | Send the window to a workspace, or the one beside |
| M, Del | Minimise, close |
| Esc | Back, with nothing changed |

## Run or raise

One key per app: it goes to the app's own workspace and brings its window forward, or starts the app there if it isn't running. Only that workspace is searched, so the same app open somewhere else is left alone. A window the key started goes to the app's workspace even when it appears after you've moved on, and even when it comes from an app already running elsewhere, as a browser's or Ghostty's new windows do. Pressing the key again while the app is still starting waits for it rather than starting a second copy. If the app is minimised, it comes back onto its workspace.

The apps are listed in `~/.config/slipstream/settings.toml`, and a change applies as soon as the file is saved:

```toml
[run-or-raise]
raise = "ctrl+super+alt"   # the chord; with Shift added, it sends the focused window instead

[[run-or-raise.app]]
workspace = "Chrome"       # a workspace's name, from [[workspaces.list]]
key = "c"
app = "browser"            # browser, terminal or files: works out the class, and starts that app
command = "chromium --new-window"   # optional with app; run with sh -c

[[run-or-raise.app]]
workspace = "Emacs"
key = "e"
class = "emacs"            # the window's app id (X11: class), any case
title = ""                 # or its exact title, when the class isn't enough
command = "emacsclient --alternate-editor= --create-frame"
```

| Keys | What they do |
|---|---|
| The chord and an app's key | Go to the app's workspace and raise it, or start it there |
| The chord, Shift and an app's key | Send the focused window to the app's workspace, and follow it |
| The chord and 1–0 | Go to a workspace by its number |
| The chord, Shift and 1–0 | Send the focused window to a workspace by its number |

Ctrl+Super+Alt is the "hyper" chord a keyd layer on one key gives, for example right Alt as `[hyper_mod:C-M-A]`. A run-or-raise key takes over a built-in key on the same chord. Each app appears on the Super+K list under its workspace's name.

A workspace can be pinned to a screen by its connector name, and then always opens there, taking the keyboard with it. The other screens' bars leave it out, and their Super+Tab passes over it. If that screen isn't connected, the workspace opens wherever the keyboard is:

```toml
[[workspaces.list]]
id = 6
screen = "HDMI-A-1"
```

Super+1–0 go to the first ten workspaces in the list, so named run-or-raise workspaces go after those ten.
