use raw_rabbitmq;
use error::Error;
use std::ffi::{CStr, CString};
use types::channel::Channel;
use types::exchange::Exchange;
use libc::c_char;

#[derive(Debug)]
pub struct Queue {
    pub name: String,
    pub passive: bool,
    pub durable: bool,
    pub exclusive: bool,
    pub auto_delete: bool,
    pub name_t: raw_rabbitmq::amqp_bytes_t,
}


impl Queue {
    // add code here
    pub fn new(
        channel: &Channel,
        name: String,
        passive: bool,
        durable: bool,
        exclusive: bool,
        auto_delete: bool,
    ) -> Result<Queue, Error> {
        let conn = channel.conn.ptr();
        let cstring_name = CString::new(name.as_str())?;
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

        let queue_name = unsafe { raw_rabbitmq::amqp_bytes_malloc_dup((*queue_declare_r).queue) };

        if queue_name.bytes.is_null() {
            return Err(Error::Reply);
        }

        Ok(Queue {
            name: name,
            passive: passive,
            durable: durable,
            exclusive: exclusive,
            auto_delete: auto_delete,
            name_t: queue_name,
        })
    }


    pub fn bind(
        &self,
        channel: &Channel,
        exchange: &Exchange,
        bindingkey: &str,
    ) -> Result<(), Error> {
        let bindingkey = CString::new(bindingkey)?;
        let conn = channel.conn.ptr();
        let bind_r = unsafe {
            raw_rabbitmq::amqp_queue_bind(
                conn,
                channel.id,
                self.name_t,
                exchange.name_t,
                raw_rabbitmq::amqp_cstring_bytes(bindingkey.as_ptr()),
                raw_rabbitmq::amqp_empty_table,
            );
        };

        let reply = unsafe { raw_rabbitmq::amqp_get_rpc_reply(conn) };
        match reply.reply_type {
            raw_rabbitmq::amqp_response_type_enum__AMQP_RESPONSE_NORMAL => Ok(()),
            _ => Err(Error::Reply),
        }
    }
}
