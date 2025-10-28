# FastStreamDB
A Bancho packet stream database built with performance in mind.

## Rationale
The main difficulty with scaling a Bancho horizontally are the packets streams. There exists no specialised solution for this (to my knowledge), and others come with much overhead.
The objective of FastStreamDB is to facilitate this niche by providing a database optimised specifically for those operations and their frequencies.

- Create new stream.
- Delete new stream.
- Enqueue to singular stream (most common).
- Enqueue to ALL streams (very common).
- Enqueue to multiple streams (common but less so).
- Fetch all current enqueued bytes for a single stream and clear the buffer. (relatively rare, once every 2-3s per stream)

## Implementation
FastStreamDB exists as a standalone client-server database written in Rust, meant to be ran in a Docker container.

### Configuration
FastStreamDB features some basic configuration done through environment variables.

| Name | Description | Default |
|------|-------------|---------|
| `FSDB_KEY_EXPIRY` | The time (in seconds) after which the streams should be considered "idle" and deleted. Set to 0 for never. | `60` |
| `FSDB_CONNECTION_MODE` | The protocol through which the server should be accessible. Either `UNIX_SOCK` or `TCP` | `UNIX_SOCK` |
| `FSDB_UNIX_SOCK_PATH` | The path on which the UNIX socket should be open. Has no effect if `FSDB_CONNECTION_MODE` is set to `TCP`. | `/tmp/fsdb.sock` |
| `FSDB_TCP_PORT` | The port on which the TCP server should listen. Has no effect if `FSDB_CONNECTION_MODE` is set to `UNIX_SOCK`. | `1273` |
| `FSDB_TCP_HOST` | The TCP host on which the server should listen. Has no effect if `FSDB_CONNECTION_MODE` is set to `UNIX_SOCK`. | `127.0.0.1` |
