#!/bin/bash
# Evalúa JS en el gnome-shell anidado.
export DBUS_SESSION_BUS_ADDRESS=$(cat "$(dirname "$0")"/bus.env)
gdbus call --session --dest org.gnome.Shell --object-path /org/gnome/Shell --method org.gnome.Shell.Eval "$1"
