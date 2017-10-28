use raw_rabbitmq;
use error::Error;
use std::ffi::{CStr, CString};
use types::channel::Channel;
use libc::c_char;


#[derive(Debug)]
pub struct Queue<'a> {
    pub channel: &'a Channel<'a>,
    pub name: &'a str,
    pub passive: bool,
    pub durable: bool,
    pub exclusive: bool,
    pub auto_delete: bool,
    pub name_t: raw_rabbitmq::amqp_bytes_t,
}


impl<'a> Queue<'a> {
    // add code here
    pub fn new(
        channel: &'a Channel<'a>,
        name: &'a str,
        passive: bool,
        durable: bool,
        exclusive: bool,
        auto_delete: bool,
    ) -> Result<Queue<'a>, Error> {
        let conn = channel.conn.ptr();
        let cstring_name = CString::new(name)?;
        let cstring_name_bytes = unsafe { raw_rabbitmq::amqp_cstring_bytes(cstring_name.as_ptr()) };
        let queue_declare_r = unsafe {
            raw_rabbitmq::amqp_queue_declare(
                conn,
                channel.id,
                cstring_name_bytes,
                passive as i32,
                durable as i32,
                exclusive as i32,
                auto_delete as i32,
                raw_rabbitmq::amqp_empty_table,
            )
        };
        let reply = unsafe { raw_rabbitmq::amqp_get_rpc_reply(conn) };

        let _ = match reply.reply_type {
            raw_rabbitmq::amqp_response_type_enum__AMQP_RESPONSE_NORMAL => Ok(()),
            _ => Err(Error::Reply),
        }?;

        let reply_to_queue =
            unsafe { raw_rabbitmq::amqp_bytes_malloc_dup((*queue_declare_r).queue) };

        if reply_to_queue.bytes.is_null() {
            return Err(Error::Reply);
        }

        Ok(Queue {
            channel: channel,
            name: name,
            passive: passive,
            durable: durable,
            exclusive: exclusive,
            auto_delete: auto_delete,
            name_t: reply_to_queue,
        })
    }
}
