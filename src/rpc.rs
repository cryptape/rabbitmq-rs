use raw_rabbitmq;
use error::Error;
use types::channel::Channel;
use types::props::BasicProperties;
use types::queue::Queue;
use types::exchange::Exchange;
use std::mem;
use libc::{self, c_char};
use std::ptr;
use util::{cstring_bytes, decode_raw_bytes, encode_bytes};
use bytes::{BufMut, BytesMut};
use rpc_message::RpcMessageFuture;
use futures::stream::Stream;
use futures::{future, Future, IntoFuture};
use bytes::Bytes;


const AMQP_BASIC_DELIVER_METHOD: u32 = 0x003C003C;

pub fn rpc_call(
    channel: &Channel,
    exchange: &Exchange,
    reply_queue: &Queue,
    routing_key: &str,
    correlation_id: &str,
    msg: Bytes,
) -> Box<Future<Item = BytesMut, Error = Error>> {
    let conn = channel.conn.ptr();
    let props = BasicProperties::new();
    let raw_props = props.raw;
    let channel_id = channel.id;

    let correlation_id = cstring_bytes(correlation_id);
    let correlation_id =
        unsafe { raw_rabbitmq::amqp_cstring_bytes(correlation_id.as_ptr() as *const c_char) };
    let content_type =
        unsafe { raw_rabbitmq::amqp_cstring_bytes(b"text/plain\0".as_ptr() as *const c_char) };

    let publish_future = unsafe {
        (*raw_props)._flags = raw_rabbitmq::AMQP_BASIC_CONTENT_TYPE_FLAG
            // | raw_rabbitmq::AMQP_BASIC_DELIVERY_MODE_FLAG
            | raw_rabbitmq::AMQP_BASIC_REPLY_TO_FLAG
            | raw_rabbitmq::AMQP_BASIC_CORRELATION_ID_FLAG;
        (*raw_props).content_type = content_type;
        // (*raw_props).delivery_mode = 2;
        (*raw_props).reply_to = raw_rabbitmq::amqp_bytes_malloc_dup(reply_queue.name_t);
        (*raw_props).correlation_id = correlation_id;

        let exchange = raw_rabbitmq::amqp_bytes_malloc_dup(exchange.name_t);

        let routing_key = cstring_bytes(routing_key);
        let publish_future = future::lazy(move || {
            let body = encode_bytes(&msg);
            let status = raw_rabbitmq::amqp_basic_publish(
                conn,
                channel_id,
                exchange,
                raw_rabbitmq::amqp_cstring_bytes(routing_key.as_ptr() as *const c_char),
                0,
                0,
                raw_props,
                body,
            );

            if status != (raw_rabbitmq::amqp_status_enum__AMQP_STATUS_OK as i32) {
                future::err(Error::Status(status))
            } else {
                future::ok(())
            }
        });

        raw_rabbitmq::amqp_bytes_free((*raw_props).reply_to);

        publish_future
    };

    // println!("basic_publish {:?}", status);

    let reply_to_queue = unsafe { raw_rabbitmq::amqp_bytes_malloc_dup(reply_queue.name_t) };
    // raw_rabbitmq::amqp_bytes_malloc_dup(reply_queue.name_t);
    // println!("{:?}", reply_to_queue);


    // // let consume_check = raw_rabbitmq::amqp_get_rpc_reply(conn);

    // // let _ = match consume_check.reply_type {
    // //     raw_rabbitmq::amqp_response_type_enum__AMQP_RESPONSE_NORMAL => Ok(()),
    // //     _ => Err(Error::Reply),
    // // }?;


    // // // let frame: *mut raw_rabbitmq::amqp_frame_t = libc::malloc(
    // // //     mem::size_of::<raw_rabbitmq::amqp_frame_t>(),
    // // // ) as *mut raw_rabbitmq::amqp_frame_t;

    // // // let mut body_target: u64 = 0;
    // // // let mut body_received: usize = 0;
    // let correlation_id = unsafe { raw_rabbitmq::amqp_cstring_bytes(correlation_id.as_ptr()) };

    let read_message = RpcMessageFuture::new(conn, channel_id, correlation_id, reply_to_queue);

    let resp = publish_future.and_then(|_| read_message);
    // let resp = resp.wait().unwrap();

    // let resp = loop {
    //     let method_frame = ReadFrame::new(conn, frame);
    //     let method_frame = method_frame.wait();
    //     if method_frame.is_err() {
    //         break None;
    //     }
    //     let frame = method_frame.unwrap();

    //     // let result = raw_rabbitmq::amqp_simple_wait_frame(conn, frame);
    //     // println!("Result:{}", result);
    //     // if result < 0 {
    //     //     break None;
    //     // }

    //     // println!(
    //     //     "Frame type: {} channel: {}",
    //     //     (*frame).frame_type,
    //     //     (*frame).channel,
    //     // );
    //     if (*frame).frame_type != (raw_rabbitmq::AMQP_FRAME_METHOD as u8) {
    //         continue;
    //     }

    //     // println!(
    //     //     "Method: {:?}",
    //     //     raw_rabbitmq::amqp_method_name((*frame).payload.method.id)
    //     // );
    //     if (*frame).payload.method.id != (AMQP_BASIC_DELIVER_METHOD) {
    //         continue;
    //     }

    //     // d = mem::transmute((*frame).payload.method.decoded);

    //     let header_frame = ReadFrame::new(conn, frame).wait();
    //     if header_frame.is_err() {
    //         break None;
    //     }
    //     let frame = header_frame.unwrap();

    //     // let result = raw_rabbitmq::amqp_simple_wait_frame(conn, frame);
    //     // if result < 0 {
    //     //     break None;
    //     // }

    //     if (*frame).frame_type != (raw_rabbitmq::AMQP_FRAME_HEADER as u8) {
    //         println!("Unexpected header!");
    //         return Err(Error::Reply);
    //     }


    //     body_target = (*frame).payload.properties.body_size;

    //     let mut buf = BytesMut::with_capacity(body_target as usize);

    //     while (body_received as u64) < body_target {
    //         // let result = raw_rabbitmq::amqp_simple_wait_frame(conn, frame);
    //         // if result < 0 {
    //         //     break;
    //         // }

    //         let body_frame = ReadFrame::new(conn, frame).wait();
    //         if body_frame.is_err() {
    //             break;
    //         }
    //         let frame = body_frame.unwrap();

    //         if (*frame).frame_type != (raw_rabbitmq::AMQP_FRAME_BODY as u8) {
    //             println!("Unexpected body!");
    //             return Err(Error::Reply);
    //         }

    //         body_received += (*frame).payload.body_fragment.len;

    //         // raw_rabbitmq::amqp_dump(
    //         //     (*frame).payload.body_fragment.bytes,
    //         //     (*frame).payload.body_fragment.len,
    //         // );

    //         let payload_body = decode_raw_bytes((*frame).payload.body_fragment);
    //         buf.put(payload_body);
    //     }

    //     if (body_received as u64) != body_target {
    //         /* Can only happen when amqp_simple_wait_frame returns <= 0 */
    //         /* We break here to close the connection */
    //         break None;
    //     }

    //     break Some(buf);
    // };
    // libc::free(frame as *mut _);
    Box::new(resp)
}
