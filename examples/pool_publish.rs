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
use rabbitmq::channel_pool::ChannelPool;

fn main() {
    let conn = Connection::new("localhost", 5672);

    assert!(conn.is_ok());
    let mut conn = conn.unwrap();
    let login = conn.login("/", 0, 131072, 0, "guest", "guest");
    assert!(login.is_ok());

    let channel = Channel::new(conn, 1002);

    assert!(channel.is_ok());
    let mut channel = channel.unwrap();

    let ex = channel.default_exchange();

    let props = BasicProperties::null();

    let pool = ChannelPool::new(conn, 3).unwrap();

    let start = PreciseTime::now();
    for i in 1..10_000_000 {
        let ch = pool.get();
        ex.publish(
            &ch,
            "test_consumer",
            false,
            false,
            &props,
            Bytes::from(format!("{}", i).as_bytes()),
        );
    }

    let end = start.to(PreciseTime::now());
    println!("{:?}", end);

    channel.close();
    conn.close();
}
