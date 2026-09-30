#!/bin/bash
# Captura la pantalla del gnome-shell anidado.
export DBUS_SESSION_BUS_ADDRESS=$(cat "$(dirname "$0")"/bus.env)
gdbus call --session --dest org.gnome.Shell.Screenshot --object-path /org/gnome/Shell/Screenshot --method org.gnome.Shell.Screenshot.Screenshot false false "$(realpath "$(dirname "$0")")/shots/$1.png"
