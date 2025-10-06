#!/bin/bash

# Simple CLI integration test script
# This script builds the CLI and runs basic smoke tests

set -e  # Exit on first error

# Build the CLI in release mode
echo "================================="
echo "Building CLI..."
echo "================================="
cargo build -p cli --release
echo ""

# Set the path to our CLI binary
CLI="./target/release/cli"

# Check that the binary was built successfully
if [ ! -f "$CLI" ]; then
    echo "ERROR: CLI binary not found at $CLI"
    exit 1
fi

echo "================================="
echo "Running Tests"
echo "================================="
echo ""

# Test 1: Basic help command should work
echo "[TEST 1] CLI --help"
$CLI --help > /dev/null
echo "✓ PASS"
echo ""

# Test 2: Container subcommand help
echo "[TEST 2] Container --help"
$CLI container --help > /dev/null
echo "✓ PASS"
echo ""

# Test 3: REPL subcommand help
echo "[TEST 3] REPL --help"
$CLI repl --help > /dev/null
echo "✓ PASS"
echo ""

# Test 4: Container create should require --image parameter
echo "[TEST 4] Container create requires --image"
if $CLI container create 2>&1 | grep -q "required"; then
    echo "✓ PASS"
else
    echo "✗ FAIL - Should require --image parameter"
    exit 1
fi
echo ""

# Test 5: REPL execute should require parameters
echo "[TEST 5] REPL execute requires parameters"
if $CLI repl execute 2>&1 | grep -q "required"; then
    echo "✓ PASS"
else
    echo "✗ FAIL - Should require parameters"
    exit 1
fi
echo ""

# Test 6: Invalid subcommand should error
echo "[TEST 6] Invalid subcommand handling"
if $CLI invalid-command 2>&1 | grep -q "unrecognized"; then
    echo "✓ PASS"
else
    echo "✗ FAIL - Should show unrecognized subcommand error"
    exit 1
fi
echo ""

# Test 7: Try to list containers (will fail gracefully if API not running)
echo "[TEST 7] Container list (without API)"
OUTPUT=$($CLI container list 2>&1 || true)
if echo "$OUTPUT" | grep -q -E "(No containers|error sending request)"; then
    echo "✓ PASS - Handled gracefully"
else
    echo "✗ FAIL - Unexpected output: $OUTPUT"
    exit 1
fi
echo ""

# Test 8: Try to list languages (will fail gracefully if API not running)
echo "[TEST 8] REPL languages (without API)"
OUTPUT=$($CLI repl languages 2>&1 || true)
if echo "$OUTPUT" | grep -q -E "(Available languages|Failed to list|error)"; then
    echo "✓ PASS - Handled gracefully"
else
    echo "✗ FAIL - Unexpected output: $OUTPUT"
    exit 1
fi
echo ""

echo "================================="
echo "All tests passed!"
echo "================================="