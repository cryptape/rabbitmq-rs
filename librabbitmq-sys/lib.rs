#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

extern crate libc;

use libc::{c_int, c_char, c_uint, size_t, c_uchar, c_void};
use std::ptr;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));


#[cfg(test)]
mod tests {
    use super::*;
    use std::mem;

    #[test]
    fn amqp_listen() {
        unsafe {
            let conn = amqp_new_connection();
            let socket = amqp_tcp_socket_new(conn);
            let status = amqp_socket_open(socket, b"localhost\0".as_ptr() as *const c_char , 5672);
            println!("status {}", status);
            let login = amqp_login(conn, b"/\0".as_ptr() as *const c_char,
                0,
                131072,
                0,
                amqp_sasl_method_enum::AMQP_SASL_METHOD_PLAIN,
                b"guest\0".as_ptr() as *const c_char,
                b"guest\0".as_ptr() as *const c_char
            );
            println!("login {:?}", login);
            let channel = amqp_channel_open(conn, 1);

            let reply = amqp_get_rpc_reply(conn);
            println!("reply {:?}", reply);

            // pub struct amqp_queue_declare_ok_t_ {
            //     pub queue: amqp_bytes_t,
            //     pub message_count: u32,
            //     pub consumer_count: u32,
            // }

            let queue_declare: *mut amqp_queue_declare_ok_t = amqp_queue_declare(conn, 1, amqp_empty_bytes, 0, 0, 0, 1,
                                 amqp_empty_table);

            println!("queue_declare {:?}", queue_declare);

            let reply = amqp_get_rpc_reply(conn);
            println!("reply {:?}", reply);

            let queue_name = amqp_bytes_malloc_dup((*queue_declare).queue);

            println!("{:?}", queue_name);

            amqp_queue_bind(conn, 1, queue_name,
                amqp_cstring_bytes(b"amq.direct\0".as_ptr() as *const c_char),
                amqp_cstring_bytes(b"test\0".as_ptr() as *const c_char),
                amqp_empty_table);

            let reply = amqp_get_rpc_reply(conn);
            println!("reply {:?}", reply);

            amqp_basic_consume(conn, 1, queue_name, amqp_empty_bytes, 0, 1, 0, amqp_empty_table);
            let reply = amqp_get_rpc_reply(conn);
            println!("reply {:?}", reply);


            // pub struct amqp_envelope_t_ {
            //     pub channel: amqp_channel_t,
            //     pub consumer_tag: amqp_bytes_t,
            //     pub delivery_tag: u64,
            //     pub redelivered: amqp_boolean_t,
            //     pub exchange: amqp_bytes_t,
            //     pub routing_key: amqp_bytes_t,
            //     pub message: amqp_message_t,
            // }

            // pub struct amqp_message_t_ {
            //     pub properties: amqp_basic_properties_t,
            //     pub body: amqp_bytes_t,
            //     pub pool: amqp_pool_t,
            // }

            let envelope: *mut amqp_envelope_t = libc::malloc(mem::size_of::<amqp_envelope_t>()) as *mut amqp_envelope_t;
            amqp_maybe_release_buffers(conn);

            let res = amqp_consume_message(conn, envelope, ptr::null_mut() , 0);
            println!("res {:?}", res);

            println!("Delivery {:?}", *envelope);

            amqp_destroy_envelope(envelope);

            println!("amqp_channel_close {:?}", amqp_channel_close(conn, 1, AMQP_REPLY_SUCCESS as c_int));
            println!("amqp_connection_close {:?}", amqp_connection_close(conn, AMQP_REPLY_SUCCESS as c_int));
            println!("amqp_destroy_connection {:?}", amqp_destroy_connection(conn));
        }
    }
}
