#!/bin/bash
# Lanza una variante del PoC dentro del gnome-shell anidado: run.sh <variante> <wayland|x11>
cd "$(dirname "$0")"
export DBUS_SESSION_BUS_ADDRESS=$(cat bus.env)
export WAYLAND_DISPLAY=rfirma-poc-0 XDG_CURRENT_DESKTOP=GNOME GDK_BACKEND=$2 POC_VARIANT=$1
[ "$2" = x11 ] && export DISPLAY=$(cat display.env) XAUTHORITY=$(ls -t /run/user/$(id -u)/.mutter-Xwaylandauth.* | head -1)
exec ./target/debug/titlebar-poc > "$1-$2.log" 2>&1
