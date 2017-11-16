use libc::timeval;
use std::time::Duration;
use bytes::Bytes;
use raw_rabbitmq::amqp_bytes_t;
use std::{mem, slice};
use libc::{self, c_char};
use error::Error;

pub fn duration_to_timeval(t: Duration) -> timeval {
    timeval {
        tv_sec: t.as_secs() as i64,
        tv_usec: (t.subsec_nanos() / 1_000_000) as i64,
    }
}

pub fn encode_bytes(bytes: &Bytes) -> amqp_bytes_t {
    amqp_bytes_t {
        len: bytes.len(),
        bytes: bytes.as_ptr() as *mut ::std::os::raw::c_void,
    }
}

pub fn decode_raw_bytes(bytes: amqp_bytes_t) -> Bytes {
    let len = bytes.len;
    let raw_ptr = bytes.bytes as *const u8;
    let slice = unsafe { slice::from_raw_parts(raw_ptr, len) };
    Bytes::from(slice)
}
