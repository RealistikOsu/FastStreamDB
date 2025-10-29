# FastStreamDB Protocol

For the purposes of efficiency, FastStreamDB uses a simple, primitive binary protocol where bytes are laid out according to a fixed schema.
- All bytes are in little endian byte order.

## Packet IDs
| Packet Name | Packet ID | Description | Has Payload |
| ----------- | --------- | ----------- | ----------- |
| `CLIENT_PING` | 0 | Prompts the server to respond with a `SERVER_PONG` packet. Used for health checking. | ❌ |
| `CLIENT_CREATE_NEW_STREAM` | 1 | Creates a new stream with a given Stream ID. Does nothing if it already exists. | ✅ |
| `CLIENT_DELETE_STREAM` | 2 | Deletes a stream with a given ID. Does nothing if it doesn't exist. | ✅ |
| `CLIENT_ENQUEUE_SINGLE` | 3 | Enqueues raw bytes to a single stream. Does nothing if it doesn't exist. | ✅ |
| `CLIENT_ENQUEUE_MULTIPLE` | 4 | Enqueues raw bytes to multiple, specified streams. Ignores non-existent streams. | ✅ |
| `CLIENT_ENQUEUE_ALL` | 5 | Enqueues raw bytes to all existing streams. | ✅ |
| `CLIENT_ENQUEUE_ALL_EXCEPT` | 6 | Enqueues raw bytes to all existing streams except the ones specified. Does not check whether the given streams exist. | ✅ |
| `CLIENT_REQUEST_STREAM_CONTENTS` | 7 | Requests the server to respond with the stream's full contents with `SERVER_STREAM_CONTENTS`, and clears them in the database. Sends an empty buffer if doesn't exist. (SUS) | ✅ |
| `CLIENT_REQUEST_STREAM_CONTENTS_NO_CLEAR` | 8 | Requests the server to respond with the stream's full contents with `SERVER_STREAM_CONTENTS`, but does not touch it's contents in the database. Sends an empty buffer if doesn't exist. (SUS) | ✅ |
| `CLIENT_CHECK_STREAM_STATE` | 9 | Requests the server to respond with `SERVER_STREAM_STATE` packet stating the stream's existence. | ✅ |
| `SERVER_PONG` | 10 | The server's way of saying it is healthy. Only sent after receiving `CLIENT_PING`. | ❌ |
| `SERVER_STREAM_CONTENTS` | 11 | The full buffer contents for a specific stream. Only sent after receiving a request from the client. | ✅ |
| `SERVER_STREAM_STATE` | 12 | States whether the stream already exists or not. Only sent after receiving `CLIENT_CHECK_STREAM_STATE`. | ✅ |

## Structures
All packets (both client and server) follow the following base structure.

| Name | Description | Size (bytes) | Data Type |
| ---- | ----------- | ------------ | --------- |
| `packet_id` | The unique packet identifier, as specified in [Packet IDs](#packet-ids). | 4 | `u32` |
| **Payload** | The packet specific payload (decided by PacketID). | Depends | Depends |

### CLIENT_CREATE_NEW_STREAM
| Name | Description | Size (bytes) | Data Type |
| ---- | ----------- | ------------ | --------- |
| `stream_id` | The unique identifier for the new stream. | 4 | `u32` |

### CLIENT_DELETE_STREAM
| Name | Description | Size (bytes) | Data Type |
| ---- | ----------- | ------------ | --------- |
| `stream_id` | The unique identifier for the stream to be deleted. | 4 | `u32` |

### CLIENT_ENQUEUE_SINGLE
| Name | Description | Size (bytes) | Data Type |
| ---- | ----------- | ------------ | --------- |
| `stream_id` | The unique identifier for the stream to be enqueued to. | 4 | `u32` |
| `enqueue_size` | The size of the data to be enqueued. | 4 | `u32` |
| `enqueue_data` | The raw bytes of size `enqueue_size` to be enqueued. | `enqueue_size` | `u8[]` |

### CLIENT_ENQUEUE_MULTIPLE
| Name | Description | Size (bytes) | Data Type |
| ---- | ----------- | ------------ | --------- |
| `enqueue_size` | The size of the data to be enqueued. | 4 | `u32` |
| `enqueue_data` | The raw bytes of size `enqueue_size` to be enqueued. | `enqueue_size` | `u8[]` |
| `filter_size` | The number of streams that the enqueue should be done to. | 4 | `u32` |
| `filter_stream_ids` | The stream IDs that should be enqueued to, of length `filter_size` | `filter_size * 4` | `u32[]` |

### CLIENT_ENQUEUE_ALL
| Name | Description | Size (bytes) | Data Type |
| ---- | ----------- | ------------ | --------- |
| `enqueue_size` | The size of the data to be enqueued. | 4 | `u32` |
| `enqueue_data` | The raw bytes of size `enqueue_size` to be enqueued. | `enqueue_size` | `u8[]` |

### CLIENT_ENQUEUE_ALL_EXCEPT
| Name | Description | Size (bytes) | Data Type |
| ---- | ----------- | ------------ | --------- |
| `enqueue_size` | The size of the data to be enqueued. | 4 | `u32` |
| `enqueue_data` | The raw bytes of size `enqueue_size` to be enqueued. | `enqueue_size` | `u8[]` |
| `filter_size` | The number of streams that should be excluded. | 4 | `u32` |
| `filter_stream_ids` | The stream IDs that should be excluded, of length `filter_size` | `filter_size * 4` | `u32[]` |

### CLIENT_REQUEST_STREAM_CONTENTS, CLIENT_REQUEST_STREAM_CONTENTS_NO_CLEAR, and CLIENT_CHECK_STREAM_STATE
| Name | Description | Size (bytes) | Data Type |
| ---- | ----------- | ------------ | --------- |
| `stream_id` | The unique identifier for the stream. | 4 | `u32` |

### SERVER_STREAM_CONTENTS
| Name | Description | Size (bytes) | Data Type |
| ---- | ----------- | ------------ | --------- |
| `buffer_size` | The size of the data to be enqueued. | 4 | `u32` |
| `buffer_data` | The raw bytes of size `buffer_size`. | `buffer_size` | `u8[]` |

### SERVER_STREAM_STATE
| Name | Description | Size (bytes) | Data Type |
| ---- | ----------- | ------------ | --------- |
| `stream_id` | The unique identifier for the stream. | 4 | `u32` |
| `is_valid` | Boolean for whether it is a valid stream. | 1 | `u8` |
| Padding | Padding for alignment simplicity | 3 | Null |

