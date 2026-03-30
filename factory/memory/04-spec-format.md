---
tags: [spec, format, yaml, intake, scout, sample]
summary: YAML spec format — required fields, optional fields, and a complete example
---

# Spec Format

Specs are YAML files in factory/specs/. They are the factory's primary input.
Scout reads and validates them. All fields below are required.

## Fields

```yaml
id:     string   # unique identifier, snake_case
title:  string   # human-readable name
purpose: string  # one-sentence intent
scope:
  - string       # things in scope
constraints:
  - string       # things that must not happen
inputs:
  - string       # what the factory receives
outputs:
  - string       # what the factory produces
acceptance_criteria:
  - string       # visible pass/fail conditions (Bramble validates)
forbidden_behaviors:
  - string       # things that must never occur (Keeper enforces)
rollback_requirements:
  - string       # what must survive if the run fails
dependencies:
  - string       # external packages, services, or tools required
performance_expectations:
  - string       # timing or throughput targets
security_expectations:
  - string       # auth, secrets, isolation requirements
```

## Complete Example

```yaml
id: user-auth-feature
title: User Authentication
purpose: Add JWT-based login and session management to the API
scope:
  - login endpoint
  - token validation middleware
  - logout endpoint
constraints:
  - no changes to existing user data schema
  - must not break existing API contracts
inputs:
  - yaml spec
  - product: sample-app
outputs:
  - implemented auth endpoints
  - visible test suite
  - artifact bundle
acceptance_criteria:
  - POST /login returns 200 with valid JWT on correct credentials
  - invalid credentials return 401
  - protected routes reject requests without valid token
forbidden_behaviors:
  - storing plaintext passwords
  - logging JWT tokens
  - path escape from workspace
rollback_requirements:
  - prior artifacts retained unless explicitly cleaned
dependencies:
  - jsonwebtoken
  - bcrypt
performance_expectations:
  - login endpoint responds within 200ms
security_expectations:
  - JWT secret loaded from env var, never hardcoded
  - bcrypt cost factor >= 10
```

## Running a Spec

    cargo run -- spec validate factory/specs/my-spec.yaml
    cargo run -- run start factory/specs/my-spec.yaml --product my-app
