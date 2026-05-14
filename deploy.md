# Deploy Dadrock Preview to a DigitalOcean Droplet

This is the simplest testing deployment:

- build the Linux binary locally with Docker;
- upload only the binary and the decisions directory;
- run `dadrock preview` publicly on the droplet with `tmux`;
- no Caddy, no HTTPS, no built-in authentication.

The example droplet IP is:

```text
165.227.39.158
```

Warning: this exposes the Dadrock preview UI to the public web. The preview server has mutating endpoints and no auth layer. Use this only for short testing, and stop it when done.

## 1. Prerequisites

On the Mac:

- Docker Desktop is installed and running.
- SSH works as `root@165.227.39.158`.
- The local Dadrock repo is at `/Users/vincent/repos/dadr`.
- The local decisions directory is at `/Users/vincent/repos/harvest_articles/decisions`.

On the droplet:

```bash
ssh root@165.227.39.158
apt update
apt install -y tmux
exit
```

If the droplet has `ufw` enabled, allow the preview port:

```bash
ssh root@165.227.39.158 'ufw allow 4004/tcp'
```

Also make sure the DigitalOcean cloud firewall allows inbound TCP `4004`.

## 2. Build the Linux Binary Locally

From the Mac:

```bash
cd /Users/vincent/repos/dadr
```

Build an amd64 Linux binary with Docker:

```bash
docker run --rm --platform linux/amd64 \
  -v "$PWD":/src \
  -w /src \
  -e CARGO_TARGET_DIR=/src/target-linux-amd64 \
  rust:1-bullseye \
  cargo build --release -p dadrock
```

The binary will be created at:

```text
/Users/vincent/repos/dadr/target-linux-amd64/release/dadrock
```

If the droplet is ARM instead of amd64, check with:

```bash
ssh root@165.227.39.158 'uname -m'
```

If it prints `aarch64`, rebuild with:

```bash
docker run --rm --platform linux/arm64 \
  -v "$PWD":/src \
  -w /src \
  -e CARGO_TARGET_DIR=/src/target-linux-arm64 \
  rust:1-bullseye \
  cargo build --release -p dadrock
```

Then use `target-linux-arm64/release/dadrock` in the upload command below.

## 3. Upload the Binary

From the Mac:

```bash
rsync -az target-linux-amd64/release/dadrock root@165.227.39.158:/usr/local/bin/dadrock
```

Make it executable:

```bash
ssh root@165.227.39.158 'chmod +x /usr/local/bin/dadrock'
```

Optional sanity check:

```bash
ssh root@165.227.39.158 '/usr/local/bin/dadrock help'
```

## 4. Upload the Decisions Directory

From the Mac:

```bash
rsync -az /Users/vincent/repos/harvest_articles/decisions/ root@165.227.39.158:/root/decisions/
```

The trailing slashes matter: this copies the contents of the local `decisions` directory into `/root/decisions/` on the droplet.

## 5. Run the Public Preview Server

SSH into the droplet:

```bash
ssh root@165.227.39.158
```

Start the server in `tmux`:

```bash
tmux new -s dadrock
dadrock preview --host 0.0.0.0 --port 4004 /root/decisions
```

Detach from `tmux` while leaving the server running:

```text
Ctrl-b
d
```

Open the public URL:

```text
http://165.227.39.158:4004
```

## 6. Stop the Server

On the droplet:

```bash
tmux kill-session -t dadrock
```

## 7. Redeploy After Local Changes

From the Mac:

```bash
cd /Users/vincent/repos/dadr

docker run --rm --platform linux/amd64 \
  -v "$PWD":/src \
  -w /src \
  -e CARGO_TARGET_DIR=/src/target-linux-amd64 \
  rust:1-bullseye \
  cargo build --release -p dadrock

rsync -az target-linux-amd64/release/dadrock root@165.227.39.158:/usr/local/bin/dadrock
rsync -az /Users/vincent/repos/harvest_articles/decisions/ root@165.227.39.158:/root/decisions/
```

Restart the running preview:

```bash
ssh root@165.227.39.158
tmux kill-session -t dadrock
tmux new -s dadrock
dadrock preview --host 0.0.0.0 --port 4004 /root/decisions
```

