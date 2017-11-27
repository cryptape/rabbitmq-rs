extern crate bytes;
extern crate futures;
extern crate libc;
extern crate rabbitmq;
extern crate time;

use std::time::Duration;
use libc::c_char;
use time::PreciseTime;
use bytes::Bytes;

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
    let consumer = Consumer::new(config, 12, ex, vec!["test".to_owned()], "test_consumer");
    consumer.start(process);
    channel.close();
    conn.close();
}


fn process(envelope: Envelope) -> Box<Future<Item = (), Error = ()>> {
    Box::new(future::lazy(move || {
        println!("{:?}", decode_raw_bytes(envelope.load().message.body));
        future::ok(())
    }))
}
