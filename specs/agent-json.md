# Agent JSON contract

These rules govern the machine-readable surfaces consumed by harness and other agents.

JSON-1: The JSON surfaces of show, list, next, next-bundle, ready, bundles, schema, diff, doctor, blocks and deps are additive-only within a schema_version: renaming or removing fields requires a schema_version bump.

JSON-2: next --json with the default count of 1 emits a bare task object or null; --count greater than 1 emits an array.

JSON-3: eff is computed at export time, remains numeric in JSON, and is rejected as a persisted task field.

JSON-4: list and ready with --fields imply JSON output and emit a bare array of task objects containing only the requested keys, instead of the normal envelope.

## Coverage boundary

The additive-only rule (JSON-1) deliberately has no test tag: individual output-shape tests do not enforce compatibility across versions for every public surface. Doctor must report it as untested until such a test exists. Tags attest to assertions in existing tests, not exhaustive proof of a rule.
