use bytes::Bytes;
use raw_rabbitmq::amqp_bytes_t;
use std::slice;

// pub fn duration_to_timeval(t: Duration) -> timeval {
//     timeval {
//         tv_sec: t.as_secs() as i64,
//         tv_usec: i64::from(t.subsec_nanos() / 1_000_000),
//     }
// }

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

// pub fn cstring_bytes(s: &str) -> Vec<u8> {
//     let mut s = s.to_owned().into_bytes();
//     s.extend(&[0]);
//     s
// }
