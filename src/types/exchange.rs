use raw_rabbitmq::{self, amqp_bytes_t};
use error::Error;
use std::ffi::CString;
use types::channel::Channel;
use libc::c_char;
use types::props::BasicProperties;
use bytes::Bytes;
use util::encode_bytes;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExchangeType {
    Fanout,
    Direct,
    Topic,
    Headers,
}

impl ExchangeType {
    pub fn to_cstr_bytes(&self) -> amqp_bytes_t {
        let type_str = match *self {
            ExchangeType::Fanout => "fanout\0",
            ExchangeType::Direct => "direct\0",
            ExchangeType::Topic => "topic\0",
            ExchangeType::Headers => "headers\0",
        };

        unsafe { raw_rabbitmq::amqp_cstring_bytes(type_str.as_bytes().as_ptr() as *const c_char) }
    }
}


#[derive(Debug, Clone)]
pub struct Exchange {
    pub exchange_type: ExchangeType,
    pub passive: bool,
    pub durable: bool,
    pub auto_delete: bool,
    pub internal: bool,
    name_t: amqp_bytes_t,
    _cstring_name: CString,
}

unsafe impl Sync for Exchange {}
unsafe impl Send for Exchange {}

impl Default for Exchange {
    fn default() -> Exchange {
        let name = CString::new("").unwrap();
        Exchange {
            exchange_type: ExchangeType::Direct,
            passive: false,
            durable: false,
            auto_delete: false,
            internal: false,
            name_t: unsafe { raw_rabbitmq::amqp_cstring_bytes(name.as_ptr() as *const c_char) },
            _cstring_name: name,
        }
    }
}

impl Exchange {
    // add code here
    pub fn new(
        channel: Channel,
        name: &str,
        exchange_type: ExchangeType,
        passive: bool,
        durable: bool,
        auto_delete: bool,
        internal: bool,
    ) -> Result<Exchange, Error> {
        let conn = channel.conn.raw_ptr();
        let cstring_name = CString::new(name)?;
        let cstring_name_bytes = unsafe { raw_rabbitmq::amqp_cstring_bytes(cstring_name.as_ptr()) };

        let amqp_exchange_declare_ok_t = unsafe {
            raw_rabbitmq::amqp_exchange_declare(
                conn,
                channel.id,
                cstring_name_bytes,
                exchange_type.to_cstr_bytes(),
                passive as i32,
                durable as i32,
                auto_delete as i32,
                internal as i32,
                raw_rabbitmq::amqp_empty_table,
            )
        };
        let reply = unsafe { raw_rabbitmq::amqp_get_rpc_reply(conn) };

        match reply.reply_type {
            raw_rabbitmq::amqp_response_type_enum__AMQP_RESPONSE_NORMAL => Ok(()),
            _ => Err(Error::Reply),
        }?;

        Ok(Exchange {
            exchange_type: exchange_type,
            passive: passive,
            durable: durable,
            auto_delete: auto_delete,
            internal: internal,
            name_t: cstring_name_bytes,
            _cstring_name: cstring_name,
        })
    }

    pub fn name(&self) -> amqp_bytes_t {
        self.name_t
    }

    pub fn publish(
        &self,
        channel: &Channel,
        routing_key: &str,
        mandatory: bool,
        immediate: bool,
        props: &BasicProperties,
        body: Bytes,
    ) -> Result<(), Error> {
        let conn = channel.conn.raw_ptr();
        let routing_key = CString::new(routing_key)?;
        let body = encode_bytes(&body);
        let status = unsafe {
            raw_rabbitmq::amqp_basic_publish(
                conn,
                channel.id,
                self.name_t,
                raw_rabbitmq::amqp_cstring_bytes(routing_key.as_ptr()),
                mandatory as i32,
                immediate as i32,
                props.raw,
                body,
            )
        };

        if status != (raw_rabbitmq::amqp_status_enum__AMQP_STATUS_OK as i32) {
            return Err(Error::Status(status));
        }
        Ok(())
    }
}
