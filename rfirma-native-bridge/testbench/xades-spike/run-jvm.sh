#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CP="$ROOT/target/classes:$ROOT/target/xades-spike-0.1.0.jar:$(cat "$ROOT/target/cp.txt")"
/home/sergio/.sdkman/candidates/java/25.3.4+1.r25-graalce/bin/java -cp "$CP" es.gob.afirma.xadesspike.Main "$@"
