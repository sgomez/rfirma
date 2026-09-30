#!/bin/bash
# Clic (o doble clic con n=2) del botón b en (x,y): click.sh x y [n] [b]
E="$(dirname "$0")"/ev.sh
n=${3:-1}; b=${4:-1}
$E "pocMove($1+0.5,$2+0.5); 'm'" >/dev/null; sleep 0.3
for i in $(seq $n); do $E "pocBtn($b,true); 'p'" >/dev/null; sleep 0.03; $E "pocBtn($b,false); 'r'" >/dev/null; sleep 0.08; done
sleep 0.5
