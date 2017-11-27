extern crate bytes;
extern crate futures;
extern crate libc;
extern crate librabbitmq_sys as raw_rabbitmq;
extern crate rabbitmq;
extern crate time;

use std::time::Duration;
use libc::c_char;
use time::PreciseTime;
use bytes::Bytes;
use std::str;

use rabbitmq::types::props::BasicProperties;
use rabbitmq::types::exchange::ExchangeType;
use rabbitmq::types::connection::Connection;
use rabbitmq::types::channel::Channel;
use rabbitmq::config::{ConfiBuilder, Config};
use futures::{future, Future, IntoFuture};
use rabbitmq::util::decode_raw_bytes;
use std::sync::Arc;
use rabbitmq::consumer::{Consumer, Envelope};


fn main() {
    let config_builder: ConfiBuilder = Config::new().expect("config init");
    let config: Config = config_builder.try_into().expect("config init");
    let conn = Connection::new("localhost", 5672);

    assert!(conn.is_ok());
    let mut conn = conn.unwrap();
    let login = conn.login("/", 0, 131072, 0, "guest", "guest");
    assert!(login.is_ok());

    let channel = Channel::new(conn, 10);

    assert!(channel.is_ok());
    let mut channel = channel.unwrap();
    let ex = Arc::new(channel.default_exchange());
    let consumer = Consumer::new(
        config,
        13,
        ex.clone(),
        vec!["test".to_owned()],
        "rpc_server",
    );
    consumer.start(move |envelope: Envelope| {
        let channel = channel;
        let ex = ex.clone();
        let call: Box<futures::Future<Item = (), Error = ()>> = Box::new(future::lazy(move || {
            let envelope = envelope.load();
            let body = decode_raw_bytes(envelope.message.body);
            println!("{:?}", body);
            let correlation_id = envelope.message.properties.correlation_id;
            let reply_to = decode_raw_bytes(envelope.message.properties.reply_to);
            let reply_to = str::from_utf8(&reply_to).unwrap();

            let props = BasicProperties::new();
            unsafe {
                let raw_props = props.raw;
                let content_type =
                    raw_rabbitmq::amqp_cstring_bytes(b"text/plain\0".as_ptr() as *const c_char);

                (*raw_props)._flags = raw_rabbitmq::AMQP_BASIC_CONTENT_TYPE_FLAG
                    | raw_rabbitmq::AMQP_BASIC_CORRELATION_ID_FLAG;
                (*raw_props).content_type = content_type;
                (*raw_props).correlation_id = correlation_id;
            }
            ex.publish(channel, reply_to, false, false, &props, body);
            future::ok(())
        }));
        call
    });
}
