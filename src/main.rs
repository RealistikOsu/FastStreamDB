use crate::serialisation;
use crate::serialisation::{Bytes, Packet};
use crate::settings::Settings;
use crate::utils;

use anyhow::Result;
use std::{net::TcpListener, os::unix::net::UnixListener};
use std::collections::HashMap;

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

    pub fn create_new_stream(&mut self, stream_id: u32) -> Result<()> {
        self.stream_map.insert(stream_id, Stream {
            buffer: Bytes::with_capacity(1024),
            last_activity: utils::get_current_timestamp(),
        });

        Ok(())
    }
}

fn main() {
    println!("Hello, world!");
}
