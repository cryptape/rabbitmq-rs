use producer::basic::basic_publish;
use raw_rabbitmq;
use error::Error;
use std::ffi::{CStr, CString};
// use types::channel::Channel;
use types::props::BasicProperties;
use types::queue::Queue;
use std::mem;
use libc::{self, c_char};
use std::ptr;

const AMQP_BASIC_DELIVER_METHOD: u32 = 0x003C003C;


pub fn rpc_call(
    routing_key: &str,
    reply_queue: &Queue,
    exchange: &str,
    correlation_id: &str,
    msg: &str,
) -> Result<(), Error> {
    let ch = reply_queue.channel;
    let conn = ch.conn.ptr();
    let props = BasicProperties::new();
    let raw_props = props.raw;
    let correlation_id = CString::new(correlation_id)?;

    let status = unsafe {
        (*raw_props)._flags = raw_rabbitmq::AMQP_BASIC_CONTENT_TYPE_FLAG
            | raw_rabbitmq::AMQP_BASIC_DELIVERY_MODE_FLAG
            | raw_rabbitmq::AMQP_BASIC_REPLY_TO_FLAG
            | raw_rabbitmq::AMQP_BASIC_CORRELATION_ID_FLAG;

        (*raw_props).content_type =
            raw_rabbitmq::amqp_cstring_bytes(b"text/plain\0".as_ptr() as *const c_char);
        (*raw_props).delivery_mode = 2;

        (*raw_props).reply_to = raw_rabbitmq::amqp_bytes_malloc_dup(reply_queue.name_t);
        (*raw_props).correlation_id = raw_rabbitmq::amqp_cstring_bytes(correlation_id.as_ptr());

        let status = basic_publish(&ch, exchange, routing_key, false, false, &props, msg);

        raw_rabbitmq::amqp_bytes_free((*raw_props).reply_to);

        status
    };

    println!("basic_publish {:?}", status);

    unsafe {
        let reply_to_queue = raw_rabbitmq::amqp_bytes_malloc_dup(reply_queue.name_t);
        println!("{:?}", reply_to_queue);
        raw_rabbitmq::amqp_basic_consume(
            conn,
            ch.id,
            reply_to_queue,
            raw_rabbitmq::amqp_empty_bytes,
            0,
            1,
            0,
            raw_rabbitmq::amqp_empty_table,
        );

        raw_rabbitmq::amqp_get_rpc_reply(conn);
        raw_rabbitmq::amqp_bytes_free(reply_to_queue);

        let frame: *mut raw_rabbitmq::amqp_frame_t = unsafe {
            libc::malloc(mem::size_of::<raw_rabbitmq::amqp_frame_t>())
                as *mut raw_rabbitmq::amqp_frame_t
        };

        let mut d: *mut raw_rabbitmq::amqp_basic_deliver_t = ptr::null_mut();
        let mut p: *mut raw_rabbitmq::amqp_basic_properties_t = ptr::null_mut();

        let mut body_target: u64 = 0;
        let mut body_received: usize = 0;

        loop {
            raw_rabbitmq::amqp_maybe_release_buffers(conn);
            let result = raw_rabbitmq::amqp_simple_wait_frame(conn, frame);
            println!("Result:{}", result);
            if result < 0 {
                break;
            }

            println!(
                "Frame type: {} channel: {}",
                (*frame).frame_type,
                (*frame).channel,
            );
            if (*frame).frame_type != (raw_rabbitmq::AMQP_FRAME_METHOD as u8) {
                continue;
            }

            println!(
                "Method: {:?}",
                raw_rabbitmq::amqp_method_name((*frame).payload.method.id)
            );
            if (*frame).payload.method.id != (AMQP_BASIC_DELIVER_METHOD) {
                continue;
            }

            d = mem::transmute((*frame).payload.method.decoded);

            let result = raw_rabbitmq::amqp_simple_wait_frame(conn, frame);
            if result < 0 {
                break;
            }

            if (*frame).frame_type != (raw_rabbitmq::AMQP_FRAME_HEADER as u8) {
                println!("Unexpected header!");
                return Err(Error::Reply);
            }

            p = mem::transmute((*frame).payload.properties.decoded);

            println!("flgas {}", (*p)._flags);

            println!("flgas {}", raw_rabbitmq::AMQP_BASIC_CONTENT_TYPE_FLAG);
            // if ((*p)._flags as u32) & (raw_rabbitmq::AMQP_BASIC_CONTENT_TYPE_FLAG as u32) == 0 {
            //     println!(
            //         "Content-type: {} {:?}",
            //         (*p).content_type.len,
            //         (*p).content_type.bytes
            //     );
            // }
            println!(
                "Content-type: {} {:?}",
                (*p).content_type.len,
                (*p).content_type.bytes
            );
            println!("----");


            body_target = (*frame).payload.properties.body_size;

            while (body_received as u64) < body_target {
                let result = raw_rabbitmq::amqp_simple_wait_frame(conn, frame);
                if result < 0 {
                    break;
                }

                if (*frame).frame_type != (raw_rabbitmq::AMQP_FRAME_BODY as u8) {
                    println!("Unexpected body!");
                    return Err(Error::Reply);
                }

                body_received += (*frame).payload.body_fragment.len;

                // raw_rabbitmq::amqp_dump(
                //     (*frame).payload.body_fragment.bytes,
                //     (*frame).payload.body_fragment.len,
                // );

                println!(
                    "{:?}",
                    CStr::from_ptr((*frame).payload.body_fragment.bytes as *const c_char)
                );
            }

            if (body_received as u64) != body_target {
                /* Can only happen when amqp_simple_wait_frame returns <= 0 */
                /* We break here to close the connection */
                break;
            }
            break;
        }

        Ok(())
    }
}
