mod serialisation;
mod settings;
mod utils;

use serialisation::{Bytes, Packet, deserialize_packets_with_offset, serialize_packets};
use settings::Settings;
use std::collections::{HashMap, HashSet};

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

fn main() -> anyhow::Result<()> {
    let settings = Settings::get();

    Ok(())
}
