#!/bin/bash
export XDG_CURRENT_DESKTOP=GNOME
exec gnome-shell --headless --wayland --unsafe-mode --virtual-monitor 1400x900 --wayland-display rfirma-poc-0
