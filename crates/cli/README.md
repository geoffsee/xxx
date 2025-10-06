# CLI

Command-line interface for interacting with the container and REPL APIs.

## Installation

```bash
cargo build -p cli --release
```

The binary will be available at `target/release/cli`.

## Usage

### Container Commands

#### List Containers

```bash
cargo run -p cli -- container list
# or with custom API URL
cargo run -p cli -- container list --api-url http://localhost:3000
```

#### Create Container

```bash
# Create container with just an image
cargo run -p cli -- container create --image python:3.11-slim

# Create container with image and command
cargo run -p cli -- container create \
  --image python:3.11-slim \
  --command python -c "print('Hello, World!')"
```

#### Remove Container

```bash
# Remove a container by ID
cargo run -p cli -- container remove --id <container-id>

# or with custom API URL
cargo run -p cli -- container remove --id <container-id> --api-url http://localhost:3000
```

### REPL Commands

#### List Available Languages

```bash
cargo run -p cli -- repl languages
# or with custom API URL
cargo run -p cli -- repl languages --api-url http://localhost:3002
```

#### Execute Code

```bash
# Execute Python code
cargo run -p cli -- repl execute \
  --language python \
  --code "print('Hello from Python!')"

# Execute Node.js code
cargo run -p cli -- repl execute \
  --language node \
  --code "console.log('Hello from Node!')"

# Execute Ruby code
cargo run -p cli -- repl execute \
  --language ruby \
  --code "puts 'Hello from Ruby!'"
```

## Configuration

Both APIs default to localhost URLs:
- Container API: `http://localhost:3000`
- REPL API: `http://localhost:3001`

You can override these with the `--api-url` flag on each command.

## Examples

```bash
# List all containers
cargo run -p cli -- container list

# Create a Python container
cargo run -p cli -- container create --image python:3.11-slim

# Remove a container
cargo run -p cli -- container remove --id abc123

# List available REPL languages
cargo run -p cli -- repl languages

# Execute Python code
cargo run -p cli -- repl execute --language python --code "print(2 + 2)"
```