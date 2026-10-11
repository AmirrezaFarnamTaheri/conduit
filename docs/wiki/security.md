# Security

## Trust model

Agent requests, repository files, child-process output, and future provider responses are untrusted. The local runtime must not interpret instruction-like text from those sources as authorization. Review `specs/threat-model.md` before extending a boundary.

## Current controls

- `vault` rejects unsupported commands, shell-evaluation markers, path traversal, and workspaces outside the selected root.
- `edit` binds every write to a complete file hash and rejects symlink escapes.
- `store` uses content hashes and does not accept arbitrary filesystem paths as handles.
- `memory` refuses facts whose source hash changed.
- `relay` applies hard token, cost, and time ceilings with saturating arithmetic.
- CI audits dependency sources and licenses.

## Data handling

Raw command output can contain credentials, tokens, personal data, or source code. Keep `CONDUIT_HOME` access-restricted, use short retention in production designs, and never upload runtime state to third-party services. Do not add telemetry by default.

## Reporting

Do not commit credentials or proof-of-concept exploit payloads. Preserve a minimal reproduction without sensitive data, describe impact and affected commit, and report it privately to project maintainers. Security fixes need a regression test and a threat-model update.

## Known gates

OS sandboxing, secret redaction, prompt-injection taint propagation, provider egress controls, and external security audit are release requirements that cannot be represented honestly by a local placeholder.
