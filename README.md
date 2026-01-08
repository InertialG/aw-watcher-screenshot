# aw-watcher-screenshot

An [ActivityWatch](https://activitywatch.net/) watcher that captures periodic screenshots, filters unchanged screens using perceptual hashing, and optionally uploads to S3-compatible storage.

## Features

- 📸 **Automatic Screenshot Capture** - Captures from all monitors at configurable intervals
- 🔍 **Smart Filtering** - Uses dhash (perceptual hash) to skip unchanged screens
- 🔥 **Monitor Hot-Plug** - Detects monitor changes at runtime
- 💾 **WebP Compression** - Efficient lossy/lossless WebP encoding
- ☁️ **S3 Upload** - Optional upload to S3/R2/MinIO compatible storage
- 📊 **ActivityWatch Integration** - Sends heartbeat events to AW server

## Architecture

```
TimerCaptureProducer → FilterProcessor → ToWebpProcessor → S3/Passthrough → AwServerProcessor
```

## Installation

```bash
cargo build --release
```

## Configuration

Copy `config.toml.example` to `config.toml` and customize:

```toml
[trigger]
interval_secs = 2        # Screenshot interval
timeout_secs = 3600      # Stop after this duration (optional)

[capture]
force_interval_secs = 60 # Force capture even if unchanged
dhash_threshold = 10     # Hamming distance threshold (0-64)

[cache]
cache_dir = "cache"      # Local screenshot storage
webp_quality = 75        # 1-100 (100 = lossless)

[s3]
enabled = false          # Enable S3 upload
endpoint = ""
bucket = ""
access_key = ""
secret_key = ""
region = "auto"

[aw_server]
host = "localhost"
port = 5600
pulse_time = 60.0        # Recommand be >= 4x interval_secs
```

## Usage

```bash
# With default config.toml
./aw-watcher-screenshot

# With custom config
./aw-watcher-screenshot --config /path/to/config.toml
```

## Project Structure

```
crates/
├── aw-watcher-screenshot/    # Main application
│   └── src/
│       ├── main.rs           # Entry point, pipeline setup
│       ├── config.rs         # Configuration parsing
│       ├── event.rs          # Event types
│       ├── worker.rs         # Producer/Processor/Consumer traits
│       └── worker_impl/
│           ├── capture.rs    # Screenshot capture (Producer)
│           ├── filter.rs     # Perceptual hash filtering
│           ├── cache.rs      # WebP encoding + local storage
│           ├── s3.rs         # S3 upload
│           ├── passthrough.rs# Bypass when S3 disabled
│           └── awserver.rs   # ActivityWatch heartbeat (Consumer)
└── aw-client-lite/           # Lightweight AW client
```

## License

MIT
