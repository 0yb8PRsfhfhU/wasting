# Wasting

A traffic quota wasting tool that downloads files from somewhere and discard them.

## Usage

```
wasting [-c <config_path>]
```

| Flag | Description                  | Default       |
|------|------------------------------|---------------|
| `-c` | Path to the TOML config file | `config.toml` |

Send `SIGHUP` to hot-reload the configuration without restarting:

```
kill -HUP <pid>
```

## Running as a systemd service

To run `wasting` as a background service that starts automatically on boot:

### 1. Create a service unit file

Create `/etc/systemd/system/wasting.service`:

```ini
[Unit]
Description=Wasting - Traffic Quota Wasting Tool
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=nobody
Group=nogroup
WorkingDirectory=/opt/wasting
ExecStart=/usr/local/bin/wasting -c /opt/wasting/config.toml
ExecReload=/bin/kill -HUP $MAINPID
Restart=on-failure
RestartSec=10s

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/opt/wasting

[Install]
WantedBy=multi-user.target
```

Adjust the paths according to your installation:

- `ExecStart`: Path to the `wasting` binary
- `WorkingDirectory` and `-c` flag: Location of your config file
- `User`/`Group`: Service account (default `nobody:nogroup`)

### 2. Install and enable the service

```bash
# Reload systemd to recognize the new service
sudo systemctl daemon-reload

# Enable the service to start on boot
sudo systemctl enable wasting

# Start the service now
sudo systemctl start wasting
```

### 3. Manage the service

```bash
# Check service status
sudo systemctl status wasting

# View logs
sudo journalctl -u wasting -f

# Reload configuration (sends SIGHUP)
sudo systemctl reload wasting

# Restart the service
sudo systemctl restart wasting

# Stop the service
sudo systemctl stop wasting

# Disable auto-start on boot
sudo systemctl disable wasting
```

## Configuration

The config file is TOML. A minimal example:

```toml
lambda = 0.001

[[source]]
url = "https://example.com/large-file.iso"
```

### Reference

| Field         | Type            | Required | Description                                                                                                                                           |
|---------------|-----------------|----------|-------------------------------------------------------------------------------------------------------------------------------------------------------|
| `lambda`      | float           | yes      | Parameter of the exponential distribution used to determine source-switching intervals (in seconds). Smaller values produce longer average intervals. |
| `source`      | array of tables | yes      | At least one download source.                                                                                                                         |
| `speed_limit` | table           | no       | Optional speed limit for downloads.                                                                                                                   |

#### `[[source]]`

| Field    | Type   | Required | Default | Description                                                                                |
|----------|--------|----------|---------|--------------------------------------------------------------------------------------------|
| `url`    | string | yes      | --      | URL to download from.                                                                      |
| `weight` | float  | no       | `1.0`   | Relative weight for random source selection. Higher weight means more likely to be picked. |

#### `[speed_limit]`

Tagged union -- set `type` to choose the variant.

**Static** -- constant speed limit:

```toml
[speed_limit]
type = "Static"
value = 1048576  # bytes/sec (1 MB/s)
```

**Dynamic** -- time-of-day schedule (step function in local time):

```toml
[speed_limit]
type = "Dynamic"

[[speed_limit.value]]
time = "00:00:00"       # midnight to 08:00 -> 512 KB/s
speed_limit = 524288

[[speed_limit.value]]
time = "08:00:00"       # 08:00 onward -> 10 MB/s
speed_limit = 10485760
```

The speed limit active at any moment is the one from the most recent time point at or before the current local time. If
the current time is before all listed points, it wraps around to the last point (carry-over from the previous day).

Set to 0 to pause the downloading when using dynamic speed limit. For static speed limit, setting to 0 is not allowed.

## How it works

1. A source is randomly chosen (weighted by `weight`).
2. A switch interval is sampled from `Exp(lambda)` and clamped to [10 seconds, 1 week].
3. The file is streamed and discarded. Compression (gzip, brotli, zstd, deflate) is negotiated automatically.
4. When the switch timer expires, the download is cancelled and a new source is picked. If the download errors out, it
   switches immediately.
5. On `SIGHUP`, the config is reloaded and the current download is interrupted.