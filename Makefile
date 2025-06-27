# Custom Language Interpreter Makefile
# Provides convenient commands for building, testing, and running the interpreter

# Variables
CARGO = cargo
BINARY_NAME = custom-lang
TARGET_DIR = target
RELEASE_DIR = $(TARGET_DIR)/release
DEBUG_DIR = $(TARGET_DIR)/debug
EXAMPLES_DIR = examples

# Default target
.PHONY: all
all: build

# Build targets
.PHONY: build
build:
	@echo "🔨 Building Custom Language Interpreter..."
	$(CARGO) build

.PHONY: release
release:
	@echo "🚀 Building release version..."
	$(CARGO) build --release

.PHONY: clean
clean:
	@echo "🧹 Cleaning build artifacts..."
	$(CARGO) clean

# Development targets
.PHONY: check
check:
	@echo "🔍 Checking code..."
	$(CARGO) check

.PHONY: test
test:
	@echo "🧪 Running tests..."
	$(CARGO) test

.PHONY: fmt
fmt:
	@echo "📝 Formatting code..."
	$(CARGO) fmt

.PHONY: clippy
clippy:
	@echo "📎 Running clippy..."
	$(CARGO) clippy

# Run targets
.PHONY: run
run: build
	@echo "⚡ Running interpreter..."
	$(CARGO) run

.PHONY: repl
repl: build
	@echo "🎮 Starting REPL..."
	$(CARGO) run -- --repl

.PHONY: repl-verbose
repl-verbose: build
	@echo "🎮 Starting REPL (verbose)..."
	$(CARGO) run -- --repl --verbose

# Example targets
.PHONY: run-examples
run-examples: build
	@echo "📚 Running all examples..."
	@echo "Running calculator example:"
	$(CARGO) run -- $(EXAMPLES_DIR)/calculator.cl
	@echo ""
	@echo "Running number game example:"
	$(CARGO) run -- $(EXAMPLES_DIR)/number_game.cl
	@echo ""
	@echo "Running test example:"
	$(CARGO) run -- $(EXAMPLES_DIR)/test.cl

.PHONY: run-calculator
run-calculator: build
	@echo "🧮 Running calculator example..."
	$(CARGO) run -- $(EXAMPLES_DIR)/calculator.cl

.PHONY: run-game
run-game: build
	@echo "🎮 Running number game example..."
	$(CARGO) run -- $(EXAMPLES_DIR)/number_game.cl

.PHONY: run-test
run-test: build
	@echo "🧪 Running test example..."
	$(CARGO) run -- $(EXAMPLES_DIR)/test.cl

.PHONY: run-math
run-math: build
	@echo "🔢 Running math playground example..."
	$(CARGO) run -- $(EXAMPLES_DIR)/math_playground_simple.cl

# Verbose execution
.PHONY: run-verbose
run-verbose: build
	@echo "⚡ Running interpreter (verbose)..."
	$(CARGO) run -- $(EXAMPLES_DIR)/test.cl --verbose

# Performance targets
.PHONY: bench
bench: release
	@echo "⏱️  Running performance benchmarks..."
	@echo "Testing with semantic analysis:"
	time $(RELEASE_DIR)/$(BINARY_NAME) $(EXAMPLES_DIR)/test.cl
	@echo ""
	@echo "Testing without semantic analysis:"
	time $(RELEASE_DIR)/$(BINARY_NAME) $(EXAMPLES_DIR)/test.cl --no-semantic

.PHONY: profile
profile: release
	@echo "📊 Profiling interpreter..."
	$(RELEASE_DIR)/$(BINARY_NAME) $(EXAMPLES_DIR)/test.cl --verbose

# Installation targets
.PHONY: install
install: release
	@echo "📦 Installing Custom Language Interpreter..."
	cp $(RELEASE_DIR)/$(BINARY_NAME) /usr/local/bin/
	@echo "✅ Installed to /usr/local/bin/$(BINARY_NAME)"

.PHONY: uninstall
uninstall:
	@echo "🗑️  Uninstalling Custom Language Interpreter..."
	rm -f /usr/local/bin/$(BINARY_NAME)
	@echo "✅ Uninstalled from /usr/local/bin/$(BINARY_NAME)"

# Documentation targets
.PHONY: docs
docs:
	@echo "📖 Generating documentation..."
	$(CARGO) doc --open

.PHONY: help
help:
	@echo "Custom Language Interpreter - Available Commands:"
	@echo ""
	@echo "🔨 Build Commands:"
	@echo "  make build      - Build debug version"
	@echo "  make release    - Build optimized release version"
	@echo "  make clean      - Clean build artifacts"
	@echo ""
	@echo "🔍 Development Commands:"
	@echo "  make check      - Check code for errors"
	@echo "  make test       - Run test suite"
	@echo "  make fmt        - Format code"
	@echo "  make clippy     - Run linter"
	@echo ""
	@echo "⚡ Run Commands:"
	@echo "  make run        - Run interpreter"
	@echo "  make repl       - Start interactive REPL"
	@echo "  make repl-verbose - Start REPL with verbose output"
	@echo ""
	@echo "📚 Example Commands:"
	@echo "  make run-examples   - Run all examples"
	@echo "  make run-calculator - Run calculator example"
	@echo "  make run-game      - Run number game example"
	@echo "  make run-test      - Run test example"
	@echo "  make run-math      - Run math playground example"
	@echo "  make run-verbose   - Run with verbose output"
	@echo ""
	@echo "⏱️  Performance Commands:"
	@echo "  make bench      - Run performance benchmarks"
	@echo "  make profile    - Profile interpreter execution"
	@echo ""
	@echo "📦 Installation Commands:"
	@echo "  make install    - Install to /usr/local/bin"
	@echo "  make uninstall  - Remove from /usr/local/bin"
	@echo ""
	@echo "📖 Documentation Commands:"
	@echo "  make docs       - Generate and open documentation"
	@echo "  make help       - Show this help message"

# File watching (requires cargo-watch)
.PHONY: watch
watch:
	@echo "👀 Watching for changes..."
	cargo watch -x build

.PHONY: watch-test
watch-test:
	@echo "👀 Watching and testing..."
	cargo watch -x test

# Quick development cycle
.PHONY: dev
dev: fmt clippy test build
	@echo "✅ Development cycle complete!"

# CI/CD simulation
.PHONY: ci
ci: fmt clippy test build run-examples
	@echo "✅ CI pipeline complete!"
