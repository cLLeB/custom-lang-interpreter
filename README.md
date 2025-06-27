# Custom Language Interpreter v0.2.0

[![CI](https://github.com/cLLeB/custom-lang-interpreter/workflows/Basic%20CI/badge.svg)](https://github.com/cLLeB/custom-lang-interpreter/actions)
[![Rust](https://img.shields.io/badge/rust-stable-brightgreen.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Version](https://img.shields.io/badge/version-0.2.0-orange.svg)](CHANGELOG.md)

A fully-featured programming language interpreter built in Rust, featuring a complete lexer, parser, AST, and execution engine with comprehensive error handling, arrays, and a REPL.

## 🚀 Features

### Core Language Features
- **Data Types**: Numbers (integers/floats), Strings, Booleans, Null, Arrays
- **Variables**: Dynamic typing with `let` declarations
- **Operators**: Arithmetic (+, -, *, /, %), Comparison (==, !=, >, <, >=, <=), Logical (&&, ||, !)
- **Control Flow**: If/else statements, while loops
- **Functions**: User-defined functions with parameters and return values
- **Recursion**: Full support for recursive function calls
- **Arrays**: Dynamic arrays with indexing `arr[0]`, literals `[1, 2, 3]`, and manipulation

### Built-in Functions
- **Math**: `abs()`, `sqrt()`, `pow()`, `min()`, `max()`
- **Utility**: `len()` (string/array length), `type()` (type inspection)
- **Arrays**: `push()`, `pop()`, `first()`, `last()`
- **I/O**: `print()` (output)

### Developer Experience
- **REPL**: Interactive Read-Eval-Print Loop for experimentation
- **Enhanced Error Messages**: Detailed error reporting with source context and visual pointers
- **Comprehensive Testing**: Full test suite with example programs
- **CLI Interface**: Command-line tool for running programs

## 📦 Installation

### Prerequisites
- Rust 1.70+ (install from [rustup.rs](https://rustup.rs/))

### Build from Source
```bash
git clone <repository-url>
cd custom-lang-interpreter
cargo build --release
```

## 🎯 Usage

### Command Line Interface
```bash
# Run a program file
cargo run -- examples/test.cl

# Run with verbose output (shows compilation stages)
cargo run -- examples/test.cl --verbose

# Skip semantic analysis for faster execution
cargo run -- examples/test.cl --no-semantic

# Start interactive REPL
cargo run -- --repl

# Start REPL with verbose mode
cargo run -- --repl --verbose

# Show comprehensive help
cargo run -- --help

# Show version information
cargo run -- --version
```

### CLI Options
- `--verbose, -v`: Enable detailed output showing compilation stages
- `--no-semantic`: Skip semantic analysis (faster but less safe)
- `--repl, -r`: Start interactive Read-Eval-Print Loop
- `--help, -h`: Display help information
- `--version, -V`: Show version information

### Example Programs
```bash
# Basic features demo
cargo run -- examples/demos/basic_features.cl

# Functions and recursion
cargo run -- examples/demos/functions_demo.cl

# Built-in functions
cargo run -- examples/demos/builtin_functions_demo.cl

# Algorithms showcase
cargo run -- examples/demos/algorithms_demo.cl
```

## 📝 Language Syntax

### Variables and Data Types
```javascript
let number = 42;
let text = "Hello, World!";
let flag = true;
let nothing = null;
```

### Control Flow
```javascript
// If/else statements
if (x > 10) {
    print "x is large";
} else {
    print "x is small";
}

// While loops
let i = 0;
while (i < 5) {
    print i;
    i = i + 1;
}
```

### Functions
```javascript
// Simple function
function greet(name) {
    return "Hello, " + name + "!";
}

// Recursive function
function factorial(n) {
    if (n <= 1) {
        return 1;
    } else {
        return n * factorial(n - 1);
    }
}

print greet("World");
print factorial(5);
```

## 📖 Language Reference

### Data Types
- **Numbers**: `42`, `3.14`, `-5`
- **Strings**: `"Hello"`, `"World"`
- **Booleans**: `true`, `false`
- **Null**: `null`
- **Arrays**: `[1, 2, 3]`, `["a", "b"]`, `[1, "hello", true]`

### Variables
```javascript
let name = "Alice";
let age = 25;
let isStudent = true;
let nothing = null;
let numbers = [1, 2, 3, 4, 5];
let mixed = [42, "hello", true];
```

### Operators

#### Arithmetic
- `+` Addition (also string concatenation)
- `-` Subtraction
- `*` Multiplication
- `/` Division
- `%` Modulo

#### Comparison
- `==` Equal
- `!=` Not equal
- `<` Less than
- `<=` Less than or equal
- `>` Greater than
- `>=` Greater than or equal

#### Logical
- `&&` Logical AND
- `||` Logical OR
- `!` Logical NOT

### Control Flow

#### If/Else Statements
```javascript
if (condition) {
    // code
} else if (other_condition) {
    // code
} else {
    // code
}
```

#### While Loops
```javascript
while (condition) {
    // code
}
```

### Functions

#### Function Declaration
```javascript
function functionName(param1, param2) {
    // function body
    return value; // optional
}
```

#### Function Call
```javascript
let result = functionName(arg1, arg2);
```

### Arrays

#### Array Creation and Access
```javascript
let empty = [];
let numbers = [1, 2, 3, 4, 5];
let mixed = [42, "hello", true, null];
let nested = [[1, 2], [3, 4]];

// Array indexing
print numbers[0];        // 1
print numbers[2];        // 3
print mixed[1];          // "hello"
print nested[0][1];      // 2

// String indexing (bonus feature)
let text = "Hello";
print text[0];           // "H"
print text[4];           // "o"
```

#### Array Length and Properties
```javascript
print len(numbers);      // 5
print len(empty);        // 0
print type(numbers);     // "array"
```

### Built-in Functions

#### Math Functions
```javascript
abs(x)          // Absolute value
sqrt(x)         // Square root
pow(x, y)       // x raised to the power of y
min(x, y)       // Minimum of two values
max(x, y)       // Maximum of two values
```

#### String Functions
```javascript
len(string)     // Length of string
```

#### Array Functions
```javascript
len(array)      // Length of array
push(array, value)  // Add element to array (returns new array)
pop(array)      // Get last element of array
first(array)    // Get first element of array
last(array)     // Get last element of array
```

#### Utility Functions
```javascript
type(value)     // Returns type as string ("number", "string", "boolean", "null", "array")
print(value)    // Output value to console
```

#### Examples
```javascript
print abs(-10);        // 10
print sqrt(16);        // 4
print pow(2, 3);       // 8
print min(5, 3);       // 3
print max(5, 3);       // 5
print len("hello");    // 5
print type(42);        // "number"
print type("text");    // "string"
print type(true);      // "boolean"
print type(null);      // "null"
print type([1, 2, 3]); // "array"

// Array examples
let arr = [10, 20, 30];
print first(arr);      // 10
print last(arr);       // 30
print push(arr, 40);   // [10, 20, 30, 40]
print pop(arr);        // 30
```

## 🧪 Testing

Run the comprehensive test suite:
```bash
# Run all tests
./run_tests.sh

# Run specific examples
cargo run -- examples/demos/basic_features.cl
cargo run -- examples/demos/functions_demo.cl
```

## 🏗️ Architecture

### Components
1. **Lexer** (`src/lexer.rs`) - Tokenizes source code
2. **Parser** (`src/parser.rs`) - Builds Abstract Syntax Tree
3. **AST** (`src/ast.rs`) - Defines language constructs
4. **Interpreter** (`src/interpreter.rs`) - Executes the program
5. **Environment** (`src/environment.rs`) - Manages variable scope
6. **Error Handling** (`src/error.rs`) - Comprehensive error reporting
7. **REPL** (`src/repl.rs`) - Interactive shell

### Error Handling
The interpreter provides comprehensive error reporting with source context:

#### Lexical Errors
```
Error: Lexical error at line 5, column 12: Unexpected character '@'

Source context:
   4 | let x = 10;
   5 | let y = @invalid;
              ^
```

#### Parse Errors
```
Error: Parse error at line 7, column 15: Expected ';' after expression

Source context:
   6 | let x = 10;
   7 | let y = 20
   8 | print x;
      ^
```

#### Semantic Errors
```
Error: Semantic error: Invalid operands for +: number and boolean

Source context:
   5 | let x = 10;
   6 | let result = x + true;
                      ^
```

#### Runtime Errors
```
Error: Runtime error: Undefined variable 'unknown_var'

Source context:
   8 | print x;
   9 | print unknown_var;
             ^
```

## 🎮 REPL Commands

In the interactive REPL:
- `help` - Show available commands and language features
- `exit` - Exit the REPL
- Any valid language expression or statement

### REPL Features
- **Persistent state**: Variables and functions persist across commands
- **Error recovery**: Syntax errors don't crash the REPL
- **History**: Use arrow keys to navigate command history
- **Multi-line support**: Functions and blocks can span multiple lines

## 🔧 Troubleshooting

### Common Issues

#### "File not found" Error
```bash
# Make sure the file exists and path is correct
ls examples/test.cl
cargo run -- examples/test.cl
```

#### Compilation Warnings
```bash
# Warnings are normal and don't affect functionality
# To see clean output, redirect stderr:
cargo run -- examples/test.cl 2>/dev/null
```

#### Performance Issues
```bash
# For faster execution, skip semantic analysis:
cargo run -- large_program.cl --no-semantic

# Use release build for better performance:
cargo build --release
./target/release/custom-lang examples/test.cl
```

### Debug Mode
```bash
# Use verbose mode to see compilation stages:
cargo run -- examples/test.cl --verbose

# This shows:
# - Tokenization results
# - Parse tree information
# - Semantic analysis status
# - Execution progress
```

## 📊 Examples

See the `examples/` directory for comprehensive demonstrations:
- `basic_features.cl` - Language fundamentals
- `functions_demo.cl` - Function definitions and recursion
- `builtin_functions_demo.cl` - Built-in function showcase
- `algorithms_demo.cl` - Complex algorithms and patterns

## 🏷️ **Repository Topics**

This repository is tagged with the following topics for discoverability:
- `rust` - Built with Rust programming language
- `interpreter` - Programming language interpreter
- `programming-language` - Custom programming language implementation
- `lexer` - Lexical analysis implementation
- `parser` - Syntax analysis implementation
- `ast` - Abstract Syntax Tree implementation
- `repl` - Read-Eval-Print Loop
- `arrays` - Dynamic array support
- `compiler` - Language compilation and execution
- `education` - Educational programming language project
- `rust-lang` - Rust language implementation
- `language-design` - Programming language design
- `semantic-analysis` - Static analysis implementation

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Ensure all tests pass
5. Submit a pull request

## 📄 License

This project is open source and available under the MIT License.

## 🎉 Acknowledgments

Built with Rust and the following excellent crates:
- `clap` - Command-line argument parsing
- `rustyline` - REPL functionality
- `thiserror` - Error handling
- `colored` - Terminal colors
