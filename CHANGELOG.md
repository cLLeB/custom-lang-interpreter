# Changelog

All notable changes to the Custom Language Interpreter will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2026-05-29

### Added
- **106 language features** across 10 implementation phases including closures, classes, pattern matching, destructuring, generators, async/await, modules, and a full standard library
- **Subcommands**: `fmt`, `lint`, `test`, `docs`, `profile`, `compile`, `debug`
- **JS transpilation**: `custom-lang compile file.cl --target js`
- **For-in / for-of** loops over arrays and objects
- **Class system** with inheritance (`extends`), getters/setters, `super`, decorators
- **Pattern matching** (`match` expressions with guards)
- **Destructuring** for arrays and objects
- **Generator functions** (`function*`, `yield`)
- **Try/catch/finally** and `throw`
- **Module system** (`import`/`export`)
- **Expanded standard library**: math, string, array, object, crypto, date/time, HTTP, regex, collections (Map, Set, PriorityQueue), functional helpers (curry, compose, memoize, pipe)

### Fixed
- Function names that are contextual keywords (`get`, `set`, `from`, etc.) now parse correctly
- `type()` calls in examples replaced with `get_type()` to avoid keyword collision
- Stack overflow test spawns with a 32 MB thread stack for Windows compatibility
- All 30 `clippy -D warnings` resolved across `interpreter.rs`, `parser.rs`, `ast.rs`, `lexer.rs`, `main.rs`

### Changed
- Version bumped to 0.3.0
- REPL version string updated to v0.3.0
- Cross-platform release workflow builds native binaries for Windows, Linux, and macOS

## [0.2.0] - 2024-12-27

### Added
- **Arrays Support**
  - Array literals: `[1, 2, 3]`, `["a", "b"]`, `[1, "hello", true]`
  - Array indexing: `arr[0]`, `arr[1]`, etc.
  - String indexing: `str[0]`, `str[1]`, etc.
  - Nested arrays: `[[1, 2], [3, 4]]`
  - Mixed-type arrays: `[42, "hello", true, null]`

- **Array Built-in Functions**
  - `push(array, value)` - Add element to array (returns new array)
  - `pop(array)` - Get last element of array
  - `first(array)` - Get first element of array
  - `last(array)` - Get last element of array
  - Enhanced `len()` function to support arrays

- **Array Operations**
  - Array concatenation with `+` operator: `[1, 2] + [3, 4]`
  - String + Array concatenation for display
  - Array truthiness (empty arrays are falsy, non-empty are truthy)
  - Type checking: `type([1, 2, 3])` returns `"array"`

- **Enhanced Features**
  - Improved semantic analysis for arrays
  - Better error messages for array operations
  - Array support in all expression contexts
  - Performance optimizations for array operations

### Enhanced
- **CLI Interface**
  - Updated version to 0.2.0
  - Enhanced welcome message with array features
  - Updated help text and documentation

- **Documentation**
  - Comprehensive array examples and tutorials
  - Updated README with array syntax and functions
  - New arrays demo program (`examples/arrays_demo.cl`)

### Technical Improvements
- Extended AST with `Array` and `Index` expression types
- Enhanced lexer with bracket tokens `[` and `]`
- Improved parser with array literal and indexing support
- Extended interpreter with array value type and operations
- Enhanced semantic analyzer with array type checking
- Comprehensive test coverage for array features

## [0.1.0] - 2024-12-27

### Added
- **Core Language Features**
  - Dynamic typing with numbers, strings, booleans, and null
  - Variable declarations with `let` keyword
  - Arithmetic operators (+, -, *, /, %)
  - Comparison operators (==, !=, <, <=, >, >=)
  - Logical operators (&&, ||, !)
  - String concatenation with + operator

- **Control Flow**
  - If/else statements with multiple conditions
  - While loops with proper scoping
  - Block statements with local scope

- **Functions**
  - User-defined functions with parameters
  - Return statements with optional values
  - Recursive function support
  - Function closures and proper scoping

- **Built-in Functions**
  - Math functions: `abs()`, `sqrt()`, `pow()`, `min()`, `max()`
  - String functions: `len()`
  - Utility functions: `type()`, `print()`

- **Lexical Analysis**
  - Complete tokenizer for all language constructs
  - Proper handling of numbers, strings, identifiers, and operators
  - Line and column tracking for error reporting

- **Syntax Analysis**
  - Recursive descent parser
  - Abstract Syntax Tree (AST) generation
  - Comprehensive error recovery

- **Semantic Analysis**
  - Type checking for expressions and operations
  - Scope resolution and variable validation
  - Function signature validation
  - Semantic error reporting with source context

- **Interpreter Engine**
  - Tree-walking interpreter
  - Environment-based variable scoping
  - Function call stack management
  - Built-in function integration

- **Error Handling**
  - Detailed error messages with source context
  - Visual error pointers showing exact error location
  - Color-coded error output
  - Multiple error types: lexical, parse, semantic, runtime

- **Interactive REPL**
  - Read-Eval-Print Loop for interactive development
  - Persistent state across commands
  - Help system with language documentation
  - Error recovery without crashing

- **Command Line Interface**
  - File execution mode
  - Interactive REPL mode
  - Verbose output option
  - Semantic analysis toggle
  - Comprehensive help system

- **Testing and Examples**
  - Comprehensive test suite
  - Example programs demonstrating all features
  - Algorithm implementations (factorial, fibonacci, etc.)
  - Error handling demonstrations

- **Documentation**
  - Complete README with usage instructions
  - Language reference and syntax guide
  - Tutorial with step-by-step examples
  - Architecture documentation
  - Troubleshooting guide

### Technical Details
- **Language**: Rust 1.70+
- **Dependencies**: 
  - `clap` for command-line parsing
  - `rustyline` for REPL functionality
  - `thiserror` for error handling
  - `colored` for terminal output
- **Architecture**: Modular design with separate lexer, parser, semantic analyzer, and interpreter
- **Performance**: Optimized for development and learning, with optional semantic analysis skip for faster execution

### File Structure
```
custom-lang-interpreter/
├── src/
│   ├── main.rs           # CLI and entry point
│   ├── lexer.rs          # Tokenization
│   ├── parser.rs         # Syntax analysis
│   ├── ast.rs            # Abstract syntax tree
│   ├── interpreter.rs    # Execution engine
│   ├── semantic.rs       # Semantic analysis
│   ├── error.rs          # Error handling
│   └── repl.rs           # Interactive shell
├── examples/
│   ├── test.cl           # Comprehensive feature test
│   ├── demos/            # Example programs
│   └── semantic_test.cl  # Error handling demo
├── tests/                # Test suite
├── README.md             # Main documentation
├── TUTORIAL.md           # Learning guide
├── CHANGELOG.md          # This file
└── Cargo.toml            # Project configuration
```

### Supported Language Constructs
- Variables: `let x = 42;`
- Functions: `function name(params) { body }`
- Control flow: `if/else`, `while`
- Expressions: arithmetic, logical, comparison
- Built-ins: math, string, utility functions
- Comments: `// single line`

### Known Limitations
- No arrays or objects (planned for future versions)
- No file I/O operations
- No module system
- No garbage collection (relies on Rust's memory management)
- No multi-threading support

### Future Roadmap
- Arrays and indexing
- Object-oriented features
- Module system and imports
- File I/O operations
- Standard library expansion
- Performance optimizations
- Debugging capabilities

---

## Development Notes

This initial release represents a complete, functional programming language interpreter suitable for:
- Learning programming language implementation
- Educational purposes
- Prototyping and experimentation
- Algorithm development and testing

The interpreter demonstrates professional software engineering practices including:
- Comprehensive error handling
- Modular architecture
- Extensive testing
- Complete documentation
- User-friendly CLI interface

For bug reports, feature requests, or contributions, please refer to the project repository.
