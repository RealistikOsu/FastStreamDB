mod serialisation;
mod settings;
mod utils;

use serialisation::{Bytes, Packet, deserialise_packets_with_offset, serialise_packets};
use settings::{ConnectionMode, Settings};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, UnixListener, UnixStream};
use tokio::sync::Mutex;
use tokio::time::{Duration, interval};

struct Stream {
    pub buffer: Bytes,
    pub last_activity: u64,
}

struct ServerState {
    stream_map: HashMap<u32, Stream>,
}

impl ServerState {
    fn new() -> Self {
        Self {
            stream_map: HashMap::with_capacity(1024),
        }
    }

    pub fn create_new_stream(&mut self, stream_id: u32) -> anyhow::Result<()> {
        self.stream_map.insert(
            stream_id,
            Stream {
                buffer: Bytes::with_capacity(1024),
                last_activity: utils::get_current_timestamp(),
            },
        );

        Ok(())
    }

    pub fn fetch_stream_contents(&mut self, stream_id: u32) -> Option<Bytes> {
        let stream = self.stream_map.get_mut(&stream_id)?;

        let stream_buffer = stream.buffer.clone();
        stream.buffer.clear();

        stream.last_activity = utils::get_current_timestamp();

        Some(stream_buffer)
    }

    pub fn fetch_stream_no_clear(&mut self, stream_id: u32) -> Option<Bytes> {
        let stream = self.stream_map.get_mut(&stream_id)?;

        let stream_buffer = stream.buffer.clone();
        stream.last_activity = utils::get_current_timestamp();

        Some(stream_buffer)
    }

    pub fn stream_exists(&self, stream_id: u32) -> bool {
        self.stream_map.contains_key(&stream_id)
    }

    pub fn delete_stream(&mut self, stream_id: u32) -> anyhow::Result<()> {
        self.stream_map.remove(&stream_id);

        Ok(())
    }

    pub fn enqueue_single(&mut self, stream_id: u32, data: &Bytes) -> anyhow::Result<()> {
        if let Some(stream) = self.stream_map.get_mut(&stream_id) {
            stream.buffer.extend_from_slice(data);
            stream.last_activity = utils::get_current_timestamp();
        }
        Ok(())
    }

    pub fn enqueue_multiple(&mut self, stream_ids: &[u32], data: &Bytes) -> anyhow::Result<()> {
        let current_timestamp = utils::get_current_timestamp();
        for stream_id in stream_ids {
            if let Some(stream) = self.stream_map.get_mut(stream_id) {
                stream.buffer.extend_from_slice(data);
                stream.last_activity = current_timestamp;
            }
        }
        Ok(())
    }

    pub fn enqueue_all(&mut self, data: &Bytes) -> anyhow::Result<()> {
        let current_timestamp = utils::get_current_timestamp();
        for stream in self.stream_map.values_mut() {
            stream.buffer.extend_from_slice(data);
            stream.last_activity = current_timestamp;
        }
        Ok(())
    }

    pub fn enqueue_all_except(
        &mut self,
        exclude_stream_ids: &[u32],
        data: &Bytes,
    ) -> anyhow::Result<()> {
        let exclude_set: HashSet<u32> = exclude_stream_ids.iter().copied().collect();
        let current_timestamp = utils::get_current_timestamp();
        for (stream_id, stream) in self.stream_map.iter_mut() {
            if !exclude_set.contains(stream_id) {
                stream.buffer.extend_from_slice(data);
                stream.last_activity = current_timestamp;
            }
        }
        Ok(())
    }

    // Maintenance functions.
    pub fn prune_expired_streams(&mut self, idle_time: u64) -> anyhow::Result<()> {
        let current_timestamp = utils::get_current_timestamp();

        let expired_streams = self
            .stream_map
            .iter()
            .filter(|(_, stream)| current_timestamp - stream.last_activity > idle_time)
            .map(|(stream_id, _)| *stream_id)
            .collect::<Vec<u32>>();

        for stream_id in expired_streams {
            self.delete_stream(stream_id)?;
        }

        Ok(())
    }
}

fn handle_client_packets(
    state: &mut ServerState,
    packets: Vec<Packet>,
) -> anyhow::Result<Vec<Packet>> {
    let mut responses = Vec::new();

    for packet in packets {
        match packet {
            Packet::ClientPing => {
                responses.push(Packet::ServerPong);
            }
            Packet::ClientCreateNewStream { stream_id } => {
                state.create_new_stream(stream_id)?;
            }
            Packet::ClientDeleteStream { stream_id } => {
                state.delete_stream(stream_id)?;
            }
            Packet::ClientEnqueueSingle {
                stream_id,
                enqueue_data,
            } => {
                state.enqueue_single(stream_id, &enqueue_data)?;
            }
            Packet::ClientEnqueueMultiple {
                enqueue_data,
                filter_stream_ids,
            } => {
                state.enqueue_multiple(&filter_stream_ids, &enqueue_data)?;
            }
            Packet::ClientEnqueueAll { enqueue_data } => {
                state.enqueue_all(&enqueue_data)?;
            }
            Packet::ClientEnqueueAllExcept {
                enqueue_data,
                filter_stream_ids,
            } => {
                state.enqueue_all_except(&filter_stream_ids, &enqueue_data)?;
            }
            Packet::ClientRequestStreamContents { stream_id } => {
                let buffer_data = state.fetch_stream_contents(stream_id).unwrap_or_default();
                responses.push(Packet::ServerStreamContents { buffer_data });
            }
            Packet::ClientRequestStreamContentsNoClear { stream_id } => {
                let buffer_data = state.fetch_stream_no_clear(stream_id).unwrap_or_default();
                responses.push(Packet::ServerStreamContents { buffer_data });
            }
            Packet::ClientCheckStreamState { stream_id } => {
                let is_valid = state.stream_exists(stream_id);
                responses.push(Packet::ServerStreamState {
                    stream_id,
                    is_valid,
                });
            }
            _ => {
                return Err(anyhow::anyhow!("Received server packet from client"));
            }
        }
    }

    Ok(responses)
}

async fn handle_connection<S>(mut stream: S, state: Arc<Mutex<ServerState>>) -> anyhow::Result<()>
where
    S: AsyncReadExt + AsyncWriteExt + Unpin,
{
    let mut read_buffer = Bytes::with_capacity(4096);

    loop {
        // Read data into buffer
        let mut temp_buffer = vec![0u8; 4096];
        let bytes_read = match stream.read(&mut temp_buffer).await {
            Ok(0) => break, // Connection closed
            Ok(n) => n,
            Err(e) => {
                eprintln!("Error reading from stream: {}", e);
                break;
            }
        };

        read_buffer.extend_from_slice(&temp_buffer[..bytes_read]);

        // Try to deserialize packets from the buffer
        loop {
            match deserialise_packets_with_offset(&read_buffer) {
                Ok((packets, consumed_bytes)) => {
                    if packets.is_empty() {
                        // No complete packets yet, keep the data in buffer
                        break;
                    }

                    // Process packets
                    let mut state_guard = state.lock().await;
                    match handle_client_packets(&mut *state_guard, packets) {
                        Ok(responses) => {
                            drop(state_guard); // Release lock before I/O

                            if !responses.is_empty() {
                                let response_data = serialise_packets(&responses);
                                if let Err(e) = stream.write_all(&response_data).await {
                                    eprintln!("Error writing to stream: {}", e);
                                    return Err(e.into());
                                }
                                if let Err(e) = stream.flush().await {
                                    eprintln!("Error flushing stream: {}", e);
                                    return Err(e.into());
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Error handling packets: {}", e);
                            return Err(e);
                        }
                    }

                    // Remove consumed bytes from buffer
                    if consumed_bytes > 0 {
                        read_buffer.drain(..consumed_bytes);
                    } else {
                        break;
                    }
                }
                Err(_) => {
                    // Partial packet, keep remaining data
                    break;
                }
            }
        }

        // Prevent buffer from growing too large
        if read_buffer.len() > 64 * 1024 {
            return Err(anyhow::anyhow!("Buffer too large, possible attack"));
        }
    }

    Ok(())
}

async fn handle_tcp_connection(
    stream: TcpStream,
    state: Arc<Mutex<ServerState>>,
) -> anyhow::Result<()> {
    handle_connection(stream, state).await
}

async fn handle_unix_connection(
    stream: UnixStream,
    state: Arc<Mutex<ServerState>>,
) -> anyhow::Result<()> {
    handle_connection(stream, state).await
}

async fn cleanup_task(state: Arc<Mutex<ServerState>>, idle_time: Duration) {
    let mut interval = interval(Duration::from_secs(30));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        interval.tick().await;
        let mut state_guard = state.lock().await;
        if let Err(e) = state_guard.prune_expired_streams(idle_time.as_secs()) {
            eprintln!("Error pruning expired streams: {}", e);
        }
    }
}

async fn run_tcp_server(settings: &Settings, state: Arc<Mutex<ServerState>>) -> anyhow::Result<()> {
    let addr = format!("{}:{}", settings.tcp_host, settings.tcp_port);
    let listener = TcpListener::bind(&addr).await?;
    println!("TCP server listening on {}", addr);

    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                println!("New TCP connection from {}", addr);
                let state_clone = Arc::clone(&state);
                tokio::spawn(async move {
                    if let Err(e) = handle_tcp_connection(stream, state_clone).await {
                        eprintln!("Error handling TCP connection: {}", e);
                    }
                });
            }
            Err(e) => {
                eprintln!("Error accepting TCP connection: {}", e);
            }
        }
    }
}

async fn run_unix_server(
    settings: &Settings,
    state: Arc<Mutex<ServerState>>,
) -> anyhow::Result<()> {
    // Remove existing socket file if it exists
    let _ = std::fs::remove_file(&settings.unix_sock_path);

    let listener = UnixListener::bind(&settings.unix_sock_path)?;
    println!(
        "UNIX socket server listening on {}",
        settings.unix_sock_path
    );

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                println!("New UNIX socket connection");
                let state_clone = Arc::clone(&state);
                tokio::spawn(async move {
                    if let Err(e) = handle_unix_connection(stream, state_clone).await {
                        eprintln!("Error handling UNIX connection: {}", e);
                    }
                });
            }
            Err(e) => {
                eprintln!("Error accepting UNIX connection: {}", e);
            }
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let settings = Settings::get();
    let state = Arc::new(Mutex::new(ServerState::new()));

    // Spawn cleanup task
    let state_for_cleanup = Arc::clone(&state);
    let idle_time = settings.key_expiry;
    tokio::spawn(async move {
        cleanup_task(state_for_cleanup, idle_time).await;
    });

    // Start server based on connection mode
    match settings.connection_mode {
        ConnectionMode::Tcp => run_tcp_server(settings, state).await,
        ConnectionMode::UnixSocket => run_unix_server(settings, state).await,
    }
}
