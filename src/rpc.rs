use raw_rabbitmq;
use error::Error;
use types::channel::Channel;
use types::props::BasicProperties;
use types::queue::Queue;
use types::exchange::Exchange;
use libc::c_char;
use util::encode_bytes;
use bytes::BytesMut;
use rpc_message::RpcMessageFuture;
use futures::{future, Future};
use bytes::Bytes;
use std::ffi::CString;


// const AMQP_BASIC_DELIVER_METHOD: u32 = 0x003C_003C;

pub fn rpc_call(
    channel: Channel,
    exchange: &Exchange,
    reply_queue: &Queue,
    routing_key: &str,
    correlation_id: &str,
    msg: Bytes,
) -> Box<Future<Item = BytesMut, Error = Error> + 'static> {
    let conn = channel.conn.raw_ptr();
    let channel_id = channel.id;

    let publish_future = unsafe {
        let reply_queue = raw_rabbitmq::amqp_bytes_malloc_dup(reply_queue.name_t);
        let exchange = raw_rabbitmq::amqp_bytes_malloc_dup(exchange.name_t);
        let correlation_id = CString::new(correlation_id).unwrap();
        let routing_key = CString::new(routing_key).unwrap();

        future::lazy(move || {
            let props = BasicProperties::new();
            let raw_props = props.raw;
            let content_type = unsafe {
                raw_rabbitmq::amqp_cstring_bytes(b"text/plain\0".as_ptr() as *const c_char)
            };

            (*raw_props)._flags = raw_rabbitmq::AMQP_BASIC_CONTENT_TYPE_FLAG
            // | raw_rabbitmq::AMQP_BASIC_DELIVERY_MODE_FLAG
            | raw_rabbitmq::AMQP_BASIC_REPLY_TO_FLAG
                | raw_rabbitmq::AMQP_BASIC_CORRELATION_ID_FLAG;
            (*raw_props).content_type = content_type;
            // (*raw_props).delivery_mode = 2;
            (*raw_props).reply_to = reply_queue;
            (*raw_props).correlation_id = raw_rabbitmq::amqp_cstring_bytes(correlation_id.as_ptr());

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
            if status != (raw_rabbitmq::amqp_status_enum__AMQP_STATUS_OK as i32) {
                future::err(Error::Status(status))
            } else {
                future::ok(())
            }
        })

        // raw_rabbitmq::amqp_bytes_free((*raw_props).reply_to);
    };

    let read_message = RpcMessageFuture::new(
        conn,
        channel_id,
        correlation_id.to_owned(),
        reply_queue.name_t,
    );

    let resp = publish_future.and_then(|_| read_message);
    Box::new(resp)
}
