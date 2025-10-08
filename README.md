# xxx

daddy is watching

## What is this?
A neat cloud. Not perfect but pretty sweet.      

## Prerequisites
- podman/podman compose
- cargo (Optional: for building cli)
- bun (Optional: for web UI REPL playground) 

```shell
git clone https://github.com/geoffsee/xxx.git

./scripts/run.sh

cargo run -p cli -- repl execute \
  --language python --api-url http://localhost:3002 \
  --code "for i in range(100): print(f'Line {i}: Hello from Python!')"
```