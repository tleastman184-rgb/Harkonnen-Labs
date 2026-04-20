# Assignments

This is the repo-root fallback coordination document for cases where the live
coordination API is not running yet.

Keep this file lightweight and current. Do not preserve session transcripts,
stale claims, or historical handoffs here; those belong in runtime logs or the
coordination API, not in the public repository.

Preferred live source once the server is up:

- `GET /api/coordination/assignments`
- `GET /api/coordination/policy-events`

API shortcuts:

- `POST /api/coordination/claim`
- `POST /api/coordination/heartbeat`
- `POST /api/coordination/release`

## Active Claims

None. Add claims only while the coordination API is unavailable.

## Template

```md
### <agent-name>
Task: <short task>
Status: in-progress | blocked | stale
Claimed: <timestamp>
Last heartbeat: <timestamp>
Files:
- path/to/file
```

## Coordination Rules

1. Check this file before starting work if the live API is down.
2. Keep claims scoped to the smallest practical file set.
3. Clear stale claims and historical notes once the live API is available again.
4. Move long handoffs and investigation notes into durable docs, issues, or artifacts.
