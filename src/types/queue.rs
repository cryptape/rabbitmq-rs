use raw_rabbitmq;
use error::Error;
use std::ffi::CString;
use types::channel::Channel;
use types::exchange::Exchange;

pub struct Queue {
    pub passive: bool,
    pub durable: bool,
    pub exclusive: bool,
    pub auto_delete: bool,
    name_t: raw_rabbitmq::amqp_bytes_t,
    _cstring_name: CString,
}

// unsafe impl Sync for Queue { }

impl Drop for Queue {
    fn drop(&mut self) {
        unsafe {
            raw_rabbitmq::amqp_bytes_free(self.name_t);
        }
    }
}

impl Queue {
    pub fn new(
        channel: Channel,
        name: &str,
        passive: bool,
        durable: bool,
        exclusive: bool,
        auto_delete: bool,
    ) -> Result<Queue, Error> {
        let conn = channel.conn.raw_ptr();
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

        match reply.reply_type {
            raw_rabbitmq::amqp_response_type_enum__AMQP_RESPONSE_NORMAL => Ok(()),
            _ => Err(Error::Reply),
        }?;

        let queue_name = unsafe { raw_rabbitmq::amqp_bytes_malloc_dup((*queue_declare_r).queue) };

        if queue_name.bytes.is_null() {
            return Err(Error::Reply);
        }

        Ok(Queue {
            passive: passive,
            durable: durable,
            exclusive: exclusive,
            auto_delete: auto_delete,
            name_t: queue_name,
            _cstring_name: cstring_name,
        })
    }

    pub fn name(&self) -> raw_rabbitmq::amqp_bytes_t {
        self.name_t
    }

    pub fn bind(
        &self,
        channel: Channel,
        exchange: &Exchange,
        bindingkey: &str,
    ) -> Result<(), Error> {
        let bindingkey = CString::new(bindingkey)?;
        let conn = channel.conn.raw_ptr();
        unsafe {
            raw_rabbitmq::amqp_queue_bind(
                conn,
                channel.id,
                self.name_t,
                exchange.name(),
                raw_rabbitmq::amqp_cstring_bytes(bindingkey.as_ptr()),
                raw_rabbitmq::amqp_empty_table,
            );
        }

        let reply = unsafe { raw_rabbitmq::amqp_get_rpc_reply(conn) };
        match reply.reply_type {
            raw_rabbitmq::amqp_response_type_enum__AMQP_RESPONSE_NORMAL => Ok(()),
            _ => Err(Error::Reply),
        }
    }

    pub fn purge(&self, channel: Channel) -> Result<u32, Error> {
        let conn = channel.conn.raw_ptr();
        let purge_r = unsafe { raw_rabbitmq::amqp_queue_purge(conn, channel.id, self.name_t) };
        if purge_r.is_null() {
            Err(Error::Reply)
        } else {
            Ok(unsafe { (*purge_r).message_count })
        }
    }
}
