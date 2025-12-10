pub type Bytes = Vec<u8>;

const PACKET_ID_CLIENT_PING: u32 = 0;
const PACKET_ID_CLIENT_CREATE_NEW_STREAM: u32 = 1;
const PACKET_ID_CLIENT_DELETE_STREAM: u32 = 2;
const PACKET_ID_CLIENT_ENQUEUE_SINGLE: u32 = 3;
const PACKET_ID_CLIENT_ENQUEUE_MULTIPLE: u32 = 4;
const PACKET_ID_CLIENT_ENQUEUE_ALL: u32 = 5;
const PACKET_ID_CLIENT_ENQUEUE_ALL_EXCEPT: u32 = 6;
const PACKET_ID_CLIENT_REQUEST_STREAM_CONTENTS: u32 = 7;
const PACKET_ID_CLIENT_REQUEST_STREAM_CONTENTS_NO_CLEAR: u32 = 8;
const PACKET_ID_CLIENT_CHECK_STREAM_STATE: u32 = 9;
const PACKET_ID_SERVER_PONG: u32 = 10;
const PACKET_ID_SERVER_STREAM_CONTENTS: u32 = 11;
const PACKET_ID_SERVER_STREAM_STATE: u32 = 12;

pub enum Packet {
    ClientPing,
    ClientCreateNewStream {
        stream_id: u32,
    },
    ClientDeleteStream {
        stream_id: u32,
    },
    ClientEnqueueSingle {
        stream_id: u32,
        enqueue_data: Bytes,
    },
    ClientEnqueueMultiple {
        enqueue_data: Bytes,
        filter_stream_ids: Vec<u32>,
    },
    ClientEnqueueAll {
        enqueue_data: Bytes,
    },
    ClientEnqueueAllExcept {
        enqueue_data: Bytes,
        filter_stream_ids: Vec<u32>,
    },
    ClientRequestStreamContents {
        stream_id: u32,
    },
    ClientRequestStreamContentsNoClear {
        stream_id: u32,
    },
    ClientCheckStreamState {
        stream_id: u32,
    },
    ServerPong,
    ServerStreamContents {
        buffer_data: Bytes,
    },
    ServerStreamState {
        stream_id: u32,
        is_valid: bool,
    },
}

impl Packet {
    fn packet_id(&self) -> u32 {
        match self {
            Packet::ClientPing => PACKET_ID_CLIENT_PING,
            Packet::ClientCreateNewStream { .. } => PACKET_ID_CLIENT_CREATE_NEW_STREAM,
            Packet::ClientDeleteStream { .. } => PACKET_ID_CLIENT_DELETE_STREAM,
            Packet::ClientEnqueueSingle { .. } => PACKET_ID_CLIENT_ENQUEUE_SINGLE,
            Packet::ClientEnqueueMultiple { .. } => PACKET_ID_CLIENT_ENQUEUE_MULTIPLE,
            Packet::ClientEnqueueAll { .. } => PACKET_ID_CLIENT_ENQUEUE_ALL,
            Packet::ClientEnqueueAllExcept { .. } => PACKET_ID_CLIENT_ENQUEUE_ALL_EXCEPT,
            Packet::ClientRequestStreamContents { .. } => PACKET_ID_CLIENT_REQUEST_STREAM_CONTENTS,
            Packet::ClientRequestStreamContentsNoClear { .. } => {
                PACKET_ID_CLIENT_REQUEST_STREAM_CONTENTS_NO_CLEAR
            }
            Packet::ClientCheckStreamState { .. } => PACKET_ID_CLIENT_CHECK_STREAM_STATE,
            Packet::ServerPong => PACKET_ID_SERVER_PONG,
            Packet::ServerStreamContents { .. } => PACKET_ID_SERVER_STREAM_CONTENTS,
            Packet::ServerStreamState { .. } => PACKET_ID_SERVER_STREAM_STATE,
        }
    }
}

// Writer helper functions
fn write_stream_into_buffer(buffer: &mut Bytes, stream: &Bytes) {
    let stream_size = stream.len() as u32;
    buffer.extend_from_slice(&stream_size.to_le_bytes());
    buffer.extend_from_slice(stream);
}

fn write_filter_list_into_buffer(buffer: &mut Bytes, filter_list: &Vec<u32>) {
    let filter_list_size = filter_list.len() as u32;
    buffer.extend_from_slice(&filter_list_size.to_le_bytes());

    for filter in filter_list {
        buffer.extend_from_slice(&filter.to_le_bytes());
    }
}

fn write_boolean_into_buffer(buffer: &mut Bytes, value: bool) {
    // Write boolean as u32 (1 byte value + 3 padding bytes)
    let value = if value { 1u32 } else { 0u32 };
    buffer.extend_from_slice(&value.to_le_bytes());
}

pub fn write_packet_into_buffer(buffer: &mut Bytes, packet: &Packet) {
    buffer.extend_from_slice(&packet.packet_id().to_le_bytes());

    match packet {
        // Zero-payload, zero-length packets.
        Packet::ClientPing | Packet::ServerPong => {}

        // Simpler packets with fixed size.
        Packet::ClientCreateNewStream { stream_id } => {
            buffer.extend_from_slice(&stream_id.to_le_bytes()); // Stream ID.
        }
        Packet::ClientDeleteStream { stream_id } => {
            buffer.extend_from_slice(&stream_id.to_le_bytes()); // Stream ID.
        }
        Packet::ClientEnqueueSingle {
            stream_id,
            enqueue_data,
        } => {
            buffer.extend_from_slice(&stream_id.to_le_bytes()); // Stream ID.
            write_stream_into_buffer(buffer, enqueue_data); // Enqueue data.
        }
        Packet::ClientEnqueueMultiple {
            enqueue_data,
            filter_stream_ids,
        } => {
            write_stream_into_buffer(buffer, enqueue_data); // Enqueue data.
            write_filter_list_into_buffer(buffer, filter_stream_ids); // Filter stream IDs.
        }
        Packet::ClientEnqueueAll { enqueue_data } => {
            write_stream_into_buffer(buffer, enqueue_data); // Enqueue data.
        }
        Packet::ClientEnqueueAllExcept {
            enqueue_data,
            filter_stream_ids,
        } => {
            write_stream_into_buffer(buffer, enqueue_data); // Enqueue data.
            write_filter_list_into_buffer(buffer, filter_stream_ids); // Filter stream IDs.
        }
        Packet::ClientRequestStreamContents { stream_id } => {
            buffer.extend_from_slice(&stream_id.to_le_bytes()); // Stream ID.
        }
        Packet::ClientRequestStreamContentsNoClear { stream_id } => {
            buffer.extend_from_slice(&stream_id.to_le_bytes()); // Stream ID.
        }
        Packet::ClientCheckStreamState { stream_id } => {
            buffer.extend_from_slice(&stream_id.to_le_bytes()); // Stream ID.
        }
        Packet::ServerStreamContents { buffer_data } => {
            write_stream_into_buffer(buffer, buffer_data); // Buffer data.
        }
        Packet::ServerStreamState {
            stream_id,
            is_valid,
        } => {
            buffer.extend_from_slice(&stream_id.to_le_bytes()); // Stream ID.
            write_boolean_into_buffer(buffer, *is_valid); // Is valid.
        }
    }
}

// Reader helper functions
pub struct ReadResult<T> {
    pub value: T,
    pub new_offset: usize,
}

fn read_boolean_from_buffer(buffer: &Bytes, offset: usize) -> ReadResult<bool> {
    let value = buffer[offset] > 0;
    let new_offset = offset + 4;

    ReadResult { value, new_offset }
}

// Not sure how I feel about the results, this whole think kinda relies on trust.
fn read_stream_from_buffer(buffer: &Bytes, mut offset: usize) -> anyhow::Result<ReadResult<Bytes>> {
    let stream_size = u32::from_le_bytes(buffer[offset..offset + 4].try_into()?);
    offset += 4;

    let mut new_buffer = Vec::with_capacity(stream_size as usize);
    new_buffer.extend_from_slice(&buffer[offset..offset + stream_size as usize]);

    offset += stream_size as usize;

    Ok(ReadResult {
        value: new_buffer,
        new_offset: offset,
    })
}

fn read_filter_list_from_buffer(
    buffer: &Bytes,
    mut offset: usize,
) -> anyhow::Result<ReadResult<Vec<u32>>> {
    let filter_list_size = u32::from_le_bytes(buffer[offset..offset + 4].try_into()?);
    offset += 4; // Skip past the size field

    let mut new_list = Vec::with_capacity(filter_list_size as usize);

    for _ in 0..filter_list_size {
        let value = u32::from_le_bytes(buffer[offset..offset + 4].try_into()?);
        new_list.push(value);
        offset += 4;
    }

    Ok(ReadResult {
        value: new_list,
        new_offset: offset,
    })
}

pub fn read_packet_from_buffer(
    buffer: &Bytes,
    mut offset: usize,
) -> anyhow::Result<ReadResult<Packet>> {
    let packet_id = u32::from_le_bytes(buffer[offset..offset + 4].try_into()?);
    offset += 4;

    match packet_id {
        // No payload packets.
        PACKET_ID_CLIENT_PING => Ok(ReadResult {
            value: Packet::ClientPing,
            new_offset: offset,
        }),
        PACKET_ID_SERVER_PONG => Ok(ReadResult {
            value: Packet::ServerPong,
            new_offset: offset,
        }),
        PACKET_ID_CLIENT_CREATE_NEW_STREAM => {
            let stream_id = u32::from_le_bytes(buffer[offset..offset + 4].try_into()?);
            offset += 4;
            Ok(ReadResult {
                value: Packet::ClientCreateNewStream { stream_id },
                new_offset: offset,
            })
        }
        PACKET_ID_CLIENT_DELETE_STREAM => {
            let stream_id = u32::from_le_bytes(buffer[offset..offset + 4].try_into()?);
            offset += 4;
            Ok(ReadResult {
                value: Packet::ClientDeleteStream { stream_id },
                new_offset: offset,
            })
        }
        PACKET_ID_CLIENT_ENQUEUE_SINGLE => {
            let stream_id = u32::from_le_bytes(buffer[offset..offset + 4].try_into()?);
            offset += 4;
            let enqueue_data = read_stream_from_buffer(buffer, offset)?;
            offset = enqueue_data.new_offset;
            Ok(ReadResult {
                value: Packet::ClientEnqueueSingle {
                    stream_id,
                    enqueue_data: enqueue_data.value,
                },
                new_offset: offset,
            })
        }
        PACKET_ID_CLIENT_ENQUEUE_MULTIPLE => {
            let enqueue_data = read_stream_from_buffer(buffer, offset)?;
            offset = enqueue_data.new_offset;
            let filter_stream_ids = read_filter_list_from_buffer(buffer, offset)?;
            offset = filter_stream_ids.new_offset;
            Ok(ReadResult {
                value: Packet::ClientEnqueueMultiple {
                    enqueue_data: enqueue_data.value,
                    filter_stream_ids: filter_stream_ids.value,
                },
                new_offset: offset,
            })
        }
        PACKET_ID_CLIENT_ENQUEUE_ALL => {
            let enqueue_data = read_stream_from_buffer(buffer, offset)?;
            offset = enqueue_data.new_offset;
            Ok(ReadResult {
                value: Packet::ClientEnqueueAll {
                    enqueue_data: enqueue_data.value,
                },
                new_offset: offset,
            })
        }
        PACKET_ID_CLIENT_ENQUEUE_ALL_EXCEPT => {
            let enqueue_data = read_stream_from_buffer(buffer, offset)?;
            offset = enqueue_data.new_offset;
            let filter_stream_ids = read_filter_list_from_buffer(buffer, offset)?;
            offset = filter_stream_ids.new_offset;
            Ok(ReadResult {
                value: Packet::ClientEnqueueAllExcept {
                    enqueue_data: enqueue_data.value,
                    filter_stream_ids: filter_stream_ids.value,
                },
                new_offset: offset,
            })
        }
        PACKET_ID_CLIENT_REQUEST_STREAM_CONTENTS => {
            let stream_id = u32::from_le_bytes(buffer[offset..offset + 4].try_into()?);
            offset += 4;
            Ok(ReadResult {
                value: Packet::ClientRequestStreamContents { stream_id },
                new_offset: offset,
            })
        }
        PACKET_ID_CLIENT_REQUEST_STREAM_CONTENTS_NO_CLEAR => {
            let stream_id = u32::from_le_bytes(buffer[offset..offset + 4].try_into()?);
            offset += 4;
            Ok(ReadResult {
                value: Packet::ClientRequestStreamContentsNoClear { stream_id },
                new_offset: offset,
            })
        }
        PACKET_ID_CLIENT_CHECK_STREAM_STATE => {
            let stream_id = u32::from_le_bytes(buffer[offset..offset + 4].try_into()?);
            offset += 4;
            Ok(ReadResult {
                value: Packet::ClientCheckStreamState { stream_id },
                new_offset: offset,
            })
        }
        PACKET_ID_SERVER_STREAM_CONTENTS => {
            let buffer_data = read_stream_from_buffer(buffer, offset)?;
            offset = buffer_data.new_offset;
            Ok(ReadResult {
                value: Packet::ServerStreamContents {
                    buffer_data: buffer_data.value,
                },
                new_offset: offset,
            })
        }
        PACKET_ID_SERVER_STREAM_STATE => {
            let stream_id = u32::from_le_bytes(buffer[offset..offset + 4].try_into()?);
            offset += 4;
            let is_valid = read_boolean_from_buffer(buffer, offset);
            offset = is_valid.new_offset;
            Ok(ReadResult {
                value: Packet::ServerStreamState {
                    stream_id,
                    is_valid: is_valid.value,
                },
                new_offset: offset,
            })
        }
        _ => Err(anyhow::anyhow!("Invalid packet ID: {}", packet_id)),
    }
}

pub fn serialize_packets(packets: &[Packet]) -> Bytes {
    let mut buffer = Bytes::new();
    for packet in packets {
        write_packet_into_buffer(&mut buffer, packet);
    }
    buffer
}

pub fn deserialize_packets(buffer: &Bytes) -> anyhow::Result<Vec<Packet>> {
    let mut packets = Vec::new();
    let mut offset = 0;

    while offset < buffer.len() {
        // Check if we have at least 4 bytes for packet_id
        if buffer.len() - offset < 4 {
            break; // Not enough data for packet_id
        }

        match read_packet_from_buffer(buffer, offset) {
            Ok(result) => {
                packets.push(result.value);
                offset = result.new_offset;
            }
            Err(_) => {
                // Partial packet, stop parsing
                break;
            }
        }
    }

    Ok(packets)
}

pub fn deserialize_packets_with_offset(buffer: &Bytes) -> anyhow::Result<(Vec<Packet>, usize)> {
    let mut packets = Vec::new();
    let mut offset = 0;

    while offset < buffer.len() {
        // Check if we have at least 4 bytes for packet_id
        if buffer.len() - offset < 4 {
            break; // Not enough data for packet_id
        }

        match read_packet_from_buffer(buffer, offset) {
            Ok(result) => {
                packets.push(result.value);
                offset = result.new_offset;
            }
            Err(_) => {
                // Partial packet, stop parsing
                break;
            }
        }
    }

    Ok((packets, offset))
}
