# Conformance suite

This directory is the **executable specification** for custom-lang. Each `.cl`
program is run through the real `custom-lang` binary and its standard output is
checked against a golden `<file>.cl.out` sidecar. The corpus — not any one
engine's accidents — defines correct behavior, and every change to a golden is a
deliberate, reviewable diff.

The suite is driven by `tests/conformance.rs` and runs as part of `cargo test`,
so CI enforces it on every push.

## Layout

- `core/` — the language core that **both** execution engines must agree on
  (arithmetic, strings, booleans/logic, comparisons, control flow, functions,
  scope). These are tagged `// vm: run`.
- (more categories are added as the corpus grows — collections, classes,
  enums, error handling, …)

## Directives

Place these in the leading comment block of a `.cl` file:

| Directive | Effect |
|-----------|--------|
| `// args: <flags>` | Extra CLI flags for the interpreter run (e.g. `--no-semantic`). |
| `// vm: run` | Also execute on the bytecode VM; its output **must** equal the golden. This is how interpreter/VM **parity** is enforced. |
| `// expect-error: <substring>` | The program **must be rejected** (non-zero exit) with a diagnostic containing the substring. Used for static-checker tests; no golden, no VM run. |

## Categories

- `core/` — language core, with interpreter/VM parity (`// vm: run`).
- `features/` — broader surface run on the interpreter (arrays, objects,
  closures, error handling, enums, classes).
- `types/` — the static type checker: valid annotations that must run, plus
  `// expect-error` programs that must be rejected.

## Adding or updating programs

1. Write the `.cl` program (add directives as needed).
2. Generate/refresh goldens and **review the diff**:
   ```
   CONFORMANCE_BLESS=1 cargo test --test conformance
   ```
3. Run normally to confirm everything (including VM parity) passes:
   ```
   cargo test --test conformance
   ```

Never edit a `.cl.out` by hand — always bless, then inspect.
