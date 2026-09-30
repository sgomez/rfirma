#!/bin/bash
# Pulsa en (x1,y1), arrastra hasta (x2,y2) en 10 pasos y suelta: drag.sh x1 y1 x2 y2
E="$(dirname "$0")"/ev.sh
$E "pocMove($1+0.5,$2+0.5); 'm'" >/dev/null; sleep 0.3
$E "pocBtn(1,true); 'p'" >/dev/null; sleep 0.3
for i in 1 2 3 4 5 6 7 8 9 10; do
  x=$(( $1 + ($3-$1)*i/10 )); y=$(( $2 + ($4-$2)*i/10 ))
  $E "pocMove($x+0.5,$y+0.5); 'm'" >/dev/null; sleep 0.05
done
sleep 0.3; $E "pocBtn(1,false); 'r'" >/dev/null; sleep 0.5
