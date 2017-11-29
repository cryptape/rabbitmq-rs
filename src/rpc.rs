use raw_rabbitmq;
use error::Error;
use types::channel::Channel;
use types::props::BasicProperties;
use types::queue::Queue;
use types::exchange::Exchange;
use libc::{self, c_char};
use util::{decode_raw_bytes, encode_bytes};
use bytes::{BufMut, BytesMut};
use futures::{future, Future};
use bytes::Bytes;
use std::ffi::CString;
use std::mem;

const AMQP_BASIC_DELIVER_METHOD: u32 = 0x003C_003C;

pub fn rpc_call(
    channel: &Channel,
    exchange: &Exchange,
    reply_queue: &Queue,
    routing_key: &str,
    correlation_id: &str,
    msg: Bytes,
) -> Box<Future<Item = Option<BytesMut>, Error = Error>> {
    let conn = channel.conn.raw_ptr();
    let channel_id = channel.id;
    let reply_queue = unsafe { raw_rabbitmq::amqp_bytes_malloc_dup(reply_queue.name()) };
    let exchange = unsafe { raw_rabbitmq::amqp_bytes_malloc_dup(exchange.name()) };
    let cstr_correlation_id = CString::new(correlation_id).unwrap();
    let string_correlation_id = correlation_id.to_owned();
    let routing_key = CString::new(routing_key).unwrap();
    let lazy = future::lazy(move || {
        let props = BasicProperties::new();
        let status = unsafe {
            let raw_props = props.raw;
            let content_type =
                raw_rabbitmq::amqp_cstring_bytes(b"text/plain\0".as_ptr() as *const c_char);

            (*raw_props)._flags = raw_rabbitmq::AMQP_BASIC_CONTENT_TYPE_FLAG
            // | raw_rabbitmq::AMQP_BASIC_DELIVERY_MODE_FLAG
            | raw_rabbitmq::AMQP_BASIC_REPLY_TO_FLAG
                | raw_rabbitmq::AMQP_BASIC_CORRELATION_ID_FLAG;
            (*raw_props).content_type = content_type;
            // (*raw_props).delivery_mode = 2;
            (*raw_props).reply_to = reply_queue;
            (*raw_props).correlation_id =
                raw_rabbitmq::amqp_cstring_bytes(cstr_correlation_id.as_ptr());

            let body = encode_bytes(&msg);
            let status = raw_rabbitmq::amqp_basic_publish(
                conn,
                channel_id,
                exchange,
                raw_rabbitmq::amqp_cstring_bytes(routing_key.as_ptr()),
                0,
                0,
                raw_props,
                body,
            );
            raw_rabbitmq::amqp_bytes_free((*raw_props).reply_to);
            raw_rabbitmq::amqp_bytes_free(exchange);
            status
        };
        if status != (raw_rabbitmq::amqp_status_enum__AMQP_STATUS_OK as i32) {
            future::err(Error::Status(status))
        } else {
            match wait_message(conn,channel_id,string_correlation_id, reply_queue) {
                Ok(message) => future::ok(message),
                Err(e) => future::err(e),
            }
        }
    });
    Box::new(lazy)
}


fn wait_message(
    conn: *mut raw_rabbitmq::amqp_connection_state_t_,
    channel_id: u16,
    correlation_id: String,
    reply_queue: raw_rabbitmq::amqp_bytes_t,
) -> Result<Option<BytesMut>, Error> {
    let frame: *mut raw_rabbitmq::amqp_frame_t = unsafe {
        libc::malloc(mem::size_of::<raw_rabbitmq::amqp_frame_t>())
            as *mut raw_rabbitmq::amqp_frame_t
    };
    let mut body_target: u64 = 0;
    let mut body_received: usize = 0;
    let correlation_id = Bytes::from(correlation_id);

    unsafe {
        raw_rabbitmq::amqp_basic_consume(
            conn,
            channel_id,
            reply_queue,
            raw_rabbitmq::amqp_empty_bytes,
            0,
            1,
            0,
            raw_rabbitmq::amqp_empty_table,
        );
    }

    let message = loop {
        //method frame
        unsafe {
            raw_rabbitmq::amqp_maybe_release_buffers(conn);
            let result = raw_rabbitmq::amqp_simple_wait_frame(conn, frame);
            if result < 0 {
                break None;
            }
            if (*frame).frame_type != (raw_rabbitmq::AMQP_FRAME_METHOD as u8) {
                continue;
            }
            if (*frame).payload.method.id != (AMQP_BASIC_DELIVER_METHOD) {
                continue;
            }

            //header frame
            let result = raw_rabbitmq::amqp_simple_wait_frame(conn, frame);
            if result < 0 {
                break None;
            }
            if (*frame).frame_type != (raw_rabbitmq::AMQP_FRAME_HEADER as u8) {
                println!("Unexpected header!");
                error!("Unexpected header!");
                return Err(Error::Reply);
            }

            let props: *mut raw_rabbitmq::amqp_basic_properties_t =
                mem::transmute((*frame).payload.properties.decoded);
            let received_correlation_id = decode_raw_bytes((*props).correlation_id);

            if received_correlation_id != correlation_id {
                warn!("Unexpected correlation_id!");
                println!("Unexpected correlation_id!");
                continue;
            }
            body_target = (*frame).payload.properties.body_size;
            let mut buf = BytesMut::with_capacity(body_target as usize);

            while (body_received as u64) < body_target {
                let result = raw_rabbitmq::amqp_simple_wait_frame(conn, frame);
                if result < 0 {
                    break;
                }
                if (*frame).frame_type != (raw_rabbitmq::AMQP_FRAME_BODY as u8) {
                    error!("Unexpected body!");
                    return Err(Error::Reply);
                }
                body_received += (*frame).payload.body_fragment.len;

                let payload_body = decode_raw_bytes((*frame).payload.body_fragment);
                buf.put(payload_body);
            }

            if (body_received as u64) != body_target {
                // Can only happen when amqp_simple_wait_frame returns <= 0
                // We break here to close the connection
                break None;
            }

            break Some(buf);
        }
    };
    unsafe {
        libc::free(frame as *mut _);
    }
    Ok(message)
}
