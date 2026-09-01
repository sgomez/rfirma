#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
J=/home/sergio/.sdkman/candidates/java/25.3.4+1.r25-graalce/bin/java
P12=/home/sergio/Developer/SideProjects/rfirma/.claude/worktrees/agent-ad5516cea23af16b2/testdata/fnmt/active-rsa.p12
CP="target/probe-1.jar:$(cat target/cp.txt)"
"$J" -cp "$CP" probe.Probe three-pages.pdf "$P12" all pages-all.pdf 2>&1 | grep -v '^WARNING' | tail -5
"$J" -cp "$CP" probe.Probe three-pages.pdf "$P12" 1-2 pages-1-2.pdf 2>&1 | grep -v '^WARNING' | tail -5
ls -la ./*.pdf
