#!/bin/bash

# Test runner script for Custom Language Interpreter
echo "🚀 Custom Language Interpreter Test Suite"
echo "=========================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Counters
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# Function to run a test
run_test() {
    local test_name="$1"
    local test_file="$2"
    local expected_exit_code="${3:-0}"
    
    echo -e "\n${BLUE}Running: $test_name${NC}"
    echo "File: $test_file"
    
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    
    # Run the test
    if cargo run -- "$test_file" > /dev/null 2>&1; then
        actual_exit_code=0
    else
        actual_exit_code=1
    fi
    
    if [ $actual_exit_code -eq $expected_exit_code ]; then
        echo -e "${GREEN}✓ PASSED${NC}"
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        echo -e "${RED}✗ FAILED${NC}"
        echo "Expected exit code: $expected_exit_code, Got: $actual_exit_code"
        FAILED_TESTS=$((FAILED_TESTS + 1))
    fi
}

# Function to run a test with output verification
run_test_with_output() {
    local test_name="$1"
    local test_file="$2"
    local expected_pattern="$3"
    
    echo -e "\n${BLUE}Running: $test_name${NC}"
    echo "File: $test_file"
    
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    
    # Run the test and capture output
    output=$(cargo run -- "$test_file" 2>&1)
    exit_code=$?
    
    if [ $exit_code -eq 0 ] && echo "$output" | grep -q "$expected_pattern"; then
        echo -e "${GREEN}✓ PASSED${NC}"
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        echo -e "${RED}✗ FAILED${NC}"
        echo "Expected pattern: $expected_pattern"
        echo "Actual output:"
        echo "$output"
        FAILED_TESTS=$((FAILED_TESTS + 1))
    fi
}

# Build the project first
echo -e "${YELLOW}Building project...${NC}"
if ! cargo build --quiet; then
    echo -e "${RED}Build failed! Exiting.${NC}"
    exit 1
fi
echo -e "${GREEN}Build successful!${NC}"

# Test 1: Basic Features Demo
run_test_with_output "Basic Features Demo" "examples/demos/basic_features.cl" "Demo completed successfully"

# Test 2: Functions Demo
run_test_with_output "Functions Demo" "examples/demos/functions_demo.cl" "Functions demo completed successfully"

# Test 3: Built-in Functions Demo
run_test_with_output "Built-in Functions Demo" "examples/demos/builtin_functions_demo.cl" "Built-in functions demo completed successfully"

# Test 4: Algorithms Demo
run_test_with_output "Algorithms Demo" "examples/demos/algorithms_demo.cl" "Algorithms demo completed successfully"

# Test 5: Original Test File
run_test_with_output "Original Test" "examples/test.cl" "21"

# Test 6: Error Handling (should fail)
run_test "Error Handling Test" "examples/error_test.cl" 1

# Test 7: REPL Help (quick test)
echo -e "\n${BLUE}Testing REPL Help${NC}"
TOTAL_TESTS=$((TOTAL_TESTS + 1))
if echo "help" | cargo run -- --repl | grep -q "Custom Language Help"; then
    echo -e "${GREEN}✓ PASSED${NC}"
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    echo -e "${RED}✗ FAILED${NC}"
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi

# Test 8: Command Line Help
echo -e "\n${BLUE}Testing Command Line Help${NC}"
TOTAL_TESTS=$((TOTAL_TESTS + 1))
if cargo run -- --help | grep -q "A custom programming language interpreter"; then
    echo -e "${GREEN}✓ PASSED${NC}"
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    echo -e "${RED}✗ FAILED${NC}"
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi

# Summary
echo -e "\n${YELLOW}=========================================="
echo "Test Summary"
echo "==========================================${NC}"
echo "Total Tests: $TOTAL_TESTS"
echo -e "Passed: ${GREEN}$PASSED_TESTS${NC}"
echo -e "Failed: ${RED}$FAILED_TESTS${NC}"

if [ $FAILED_TESTS -eq 0 ]; then
    echo -e "\n${GREEN}🎉 All tests passed!${NC}"
    exit 0
else
    echo -e "\n${RED}❌ Some tests failed.${NC}"
    exit 1
fi
