# xxx

daddy is watching

## Prerequisites
- podman/podman compose
- cargo (Optional: for building cli)
- bun (Optional: for web UI REPL playground) 

```shell
git clone https://github.com/geoffsee/xxx.git

./scripts/run.sh

cargo run -p cli -- container create --api-url http://localhost:3001 \
  --image python:3.11-slim \
  --command python -c "print('Hello, World!')"
```