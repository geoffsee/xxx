#!/bin/bash

# REPL language integration test script
# Tests each supported language with a simple code example
# Requires the REPL API to be running at http://localhost:3000

# Configuration
REPL_API_URL="${REPL_API_URL:-http://localhost:3000}"
CLI="./target/release/cli"

echo "================================="
echo "Building CLI..."
echo "================================="
cargo build -p cli --release
echo ""

# Check that the binary was built successfully
if [ ! -f "$CLI" ]; then
    echo "ERROR: CLI binary not found at $CLI"
    exit 1
fi

# Check if REPL API is running
echo "Checking if REPL API is running at $REPL_API_URL..."
if ! curl -s "$REPL_API_URL/health" > /dev/null 2>&1; then
    echo "ERROR: REPL API is not running at $REPL_API_URL"
    echo "Please start the REPL API before running this test"
    exit 1
fi
echo "✓ REPL API is running"
echo ""

echo "================================="
echo "Testing REPL Languages"
echo "================================="
echo ""

# Test Python
echo "[TEST 1] Python - arithmetic"
$CLI repl execute --language python --code "print(2 + 2)" --api-url "$REPL_API_URL"
echo ""

# Test Node
echo "[TEST 2] Node - arithmetic"
$CLI repl execute --language node --code "console.log(10 * 5)" --api-url "$REPL_API_URL"
echo ""

# Test Ruby
echo "[TEST 3] Ruby - arithmetic"
$CLI repl execute --language ruby --code "puts 7 + 3" --api-url "$REPL_API_URL"
echo ""

# Test Go
echo "[TEST 4] Go - arithmetic"
$CLI repl execute --language go --code 'package main; import "fmt"; func main() { fmt.Println(15 + 5) }' --api-url "$REPL_API_URL"
echo ""

# Test Rust
echo "[TEST 5] Rust - arithmetic"
$CLI repl execute --language rust --code 'fn main() { println!("{}", 12 + 8); }' --api-url "$REPL_API_URL"
echo ""

# Test Python strings
echo "[TEST 6] Python - string operations"
$CLI repl execute --language python --code "print('Hello, World!')" --api-url "$REPL_API_URL"
echo ""

# Test Node strings
echo "[TEST 7] Node - string operations"
$CLI repl execute --language node --code "console.log('Hello, Node')" --api-url "$REPL_API_URL"
echo ""

# Test Ruby strings
echo "[TEST 8] Ruby - string operations"
$CLI repl execute --language ruby --code "puts 'Hello, Ruby'" --api-url "$REPL_API_URL"
echo ""

echo "================================="
echo "Testing Complete"
echo "================================="
echo ""
echo "All languages tested:"
echo "  • Python"
echo "  • Node"
echo "  • Ruby"
echo "  • Go"
echo "  • Rust"