#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GRAALVM_HOME="${GRAALVM_HOME:-$HOME/.sdkman/candidates/java/25.3.4+1.r25-graalce}"
CP="$ROOT/target/classes:$ROOT/target/xades-spike-0.1.0.jar:$(cat "$ROOT/target/cp.txt")"
"$GRAALVM_HOME/bin/java" -cp "$CP" es.gob.afirma.xadesspike.Validate "$@"
