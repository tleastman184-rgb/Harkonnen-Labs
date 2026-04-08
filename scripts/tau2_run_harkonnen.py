#!/usr/bin/env python3
"""Register the Harkonnen PackChat tau2 agent and delegate to tau2's CLI."""

from __future__ import annotations

import os
import sys
from pathlib import Path


def _bootstrap_paths() -> None:
    repo_root = Path(__file__).resolve().parents[1]
    tau2_root = os.environ.get("TAU2_BENCH_ROOT")
    if tau2_root:
        candidate = Path(tau2_root).resolve() / "src"
        if candidate.exists():
            sys.path.insert(0, str(candidate))
    sys.path.insert(0, str(repo_root))
    sys.path.insert(0, str(repo_root / "scripts"))


def main() -> int:
    _bootstrap_paths()

    from tau2.cli import main as tau2_main
    from tau2.registry import registry

    from tau2_harkonnen_agent import create_agent

    agent_name = os.environ.get("TAU2_BENCH_AGENT_NAME", "harkonnen_packchat")
    if registry.get_agent_factory(agent_name) is None:
        registry.register_agent_factory(create_agent, agent_name)

    if len(sys.argv) > 1 and sys.argv[1] == "run" and "--agent" not in sys.argv[2:]:
        sys.argv = [sys.argv[0], "run", "--agent", agent_name, *sys.argv[2:]]

    tau2_main()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
