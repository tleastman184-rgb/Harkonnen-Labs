#!/usr/bin/env sh
set -eu

if [ -z "${TAU2_BENCH_ROOT:-}" ]; then
  printf '%s\n' 'TAU2_BENCH_ROOT is required and should point at a tau2-bench checkout.' >&2
  exit 1
fi

export PYTHONPATH="${TAU2_BENCH_ROOT}/src:${PYTHONPATH:-}"
exec python3 "$(dirname "$0")/tau2_run_harkonnen.py" "$@"
