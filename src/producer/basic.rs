use raw_rabbitmq;
use error::Error;
use std::ffi::CString;
use types::channel::Channel;
use types::props::BasicProperties;


pub fn basic_publish(
    ch: &Channel,
    exchange: &str,
    routing_key: &str,
    mandatory: bool,
    immediate: bool,
    props: &BasicProperties,
    body: &str,
) -> Result<(), Error> {
    let conn = ch.conn.ptr();
    let exchange = CString::new(exchange)?;
    let routing_key = CString::new(routing_key)?;
    let body = CString::new(body)?;
    let status = unsafe {
        raw_rabbitmq::amqp_basic_publish(
            conn,
            ch.id,
            raw_rabbitmq::amqp_cstring_bytes(exchange.as_ptr()),
            raw_rabbitmq::amqp_cstring_bytes(routing_key.as_ptr()),
            mandatory as i32,
            immediate as i32,
            props.raw,
            raw_rabbitmq::amqp_cstring_bytes(body.as_ptr()),
        )
    };

    if status != (raw_rabbitmq::amqp_status_enum__AMQP_STATUS_OK as i32) {
        return Err(Error::Status(status));
    }
    Ok(())
}
