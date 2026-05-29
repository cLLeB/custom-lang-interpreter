# Implementation Progress

## Status Key: ✅ Done | 🔄 In Progress | ⏳ Pending

---

## Phase 1 — Core Syntax Gaps

| # | Feature | Status | Notes |
|---|---------|--------|-------|
| 1.1 | Try / Catch / Finally / Throw | ✅ | via ThrownException signal |
| 1.2 | Ternary Operator `? :` | ✅ | |
| 1.3 | Null Coalescing `??` | ✅ | |
| 1.4 | Optional Chaining `?.` | ✅ | prop & index & call |
| 1.5 | Logical Assignment `\|\|=`, `&&=`, `??=` | ✅ | |
| 1.6 | Exponentiation `**` | ✅ | right-associative |
| 1.7 | Bitwise Operators | ✅ | `&`, `\|`, `^`, `~`, `<<`, `>>`, `>>>` |
| 1.8 | `in` Operator | ✅ | |
| 1.9 | `instanceof` / `is` Operator | ✅ | |
| 1.10 | Do-While Loop | ✅ | |
| 1.11 | Labeled Break/Continue | ⏳ | complex |
| 1.12 | String Interpolation / Template Literals | ✅ | backtick \`...\` |
| 1.13 | Heredoc `""" """` | ✅ | |
| 1.14 | Raw Strings `r"..."` | ✅ | |
| 1.15 | Destructuring Assignment | ⏳ | complex |
| 1.16 | Spread / Rest Operators | ✅ | `...` in arrays & params |
| 1.17 | Default Function Parameters | ✅ | |
| 1.18 | Named / Keyword Arguments | ⏳ | complex |
| 1.19 | `super` Keyword | ✅ | method & constructor |
| 1.20 | Static Methods and Properties | ✅ | |
| 1.21 | Private Fields and Methods | ⏳ | complex |
| 1.22 | Getters and Setters | ⏳ | |
| 1.23 | Computed Property Names `{[key]: val}` | ✅ | |
| 1.24 | Shorthand Object Properties `{name, age}` | ✅ | |
| 1.25 | Method Shorthand in Objects | ⏳ | |
| 1.26 | Arrow Functions `x => x * 2` | ✅ | |
| 1.27 | Hex/Oct/Bin literals `0xFF, 0o7, 0b1` | ✅ | |

## Phase 4 — Standard Library Expansion (Selected)

| # | Module | Status | Notes |
|---|--------|--------|-------|
| 4.1 | Math Module (`math.*`) | ✅ | as builtins |
| 4.7 | JSON (`json.parse`, `json.stringify`) | ✅ | as builtins |
| 4.14 | Random (`random.*`) | ✅ | as builtins |

---

## Build Status
- Last build: 🔄 In Progress
- Tests: ⏳ Not yet run

## Changes Made
- `error.rs`: Added `ThrownException` variant
- `ast.rs`: New BinaryOp/UnaryOp/CompoundOp variants, Param struct, new Expr/Stmt variants
- `lexer.rs`: ~20 new tokens, template literals, heredoc, raw strings, 0x/0o/0b numbers
- `parser.rs`: New expression levels (ternary, null-coalesce, bitwise, shift, power), new statements
- `interpreter.rs`: New eval logic, updated class/call handling, JSON/random/math builtins
