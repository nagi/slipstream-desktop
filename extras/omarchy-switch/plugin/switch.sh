#!/bin/sh
# The Omarchy plugin: a card with one button, and a tile in quick settings, that switch to
# Omarchy through extras/omarchy-switch's desktop-switch.
set -eu
if [ "${1:-}" = press ]; then
    [ "${2:-}" = omarchy ] || { echo "switch.sh: nothing is pressed as ${2:-nothing}" >&2; exit 2; }
    switch=$HOME/.local/bin/desktop-switch
    [ -x "$switch" ] || switch=desktop-switch
    exec "$switch" omarchy
fi
echo '{"lines": ["Log out and straight back in to Omarchy"], "buttons": [{"id": "omarchy", "label": "Switch to Omarchy"}], "tile": {"id": "omarchy", "label": "Omarchy", "icon": "power", "on": false}}'
