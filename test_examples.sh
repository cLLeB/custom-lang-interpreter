#!/bin/bash

# Test script for Custom Language Interpreter examples
# This script tests all stable examples to ensure they work correctly

set -e  # Exit on any error

echo "🧪 Testing Custom Language Interpreter Examples"
echo "=============================================="

# Build the project first
echo "🔨 Building project..."
cargo build --release

# List of stable examples to test
EXAMPLES=(
    "test.cl"
    "calculator.cl"
    "arrays_demo.cl"
    "math_playground_simple.cl"
    "number_game.cl"
)

# Test each example
for example in "${EXAMPLES[@]}"; do
    echo ""
    echo "📝 Testing: $example"
    echo "----------------------------------------"
    
    if timeout 30s cargo run --release -- "examples/$example"; then
        echo "✅ $example - PASSED"
    else
        echo "❌ $example - FAILED"
        exit 1
    fi
done

echo ""
echo "🎉 All examples passed successfully!"
echo "✅ Custom Language Interpreter is working correctly"
