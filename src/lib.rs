#![feature(shared)]
extern crate bytes;
extern crate config as configlib;
extern crate futures;
extern crate libc;
extern crate librabbitmq_sys as raw_rabbitmq;
#[macro_use]
extern crate log;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate time;
extern crate tokio_core;

pub mod util;
mod error;
pub mod types;
pub mod rpc;
mod rpc_message;
pub mod config;
pub mod consumer;

#[cfg(test)]
mod tests {
    use std::time::Duration;
    use libc::c_char;
    use super::*;
    use time::PreciseTime;
    use bytes::Bytes;
    use consumer::{Consumer, Envelope};
    use types::props::BasicProperties;
    use types::exchange::ExchangeType;
    use config::{ConfiBuilder, Config};
    use futures::{future, Future, IntoFuture};
    use util::decode_raw_bytes;
    use std::sync::Arc;

    #[test]
    fn basic() {
        let conn = types::connection::Connection::new("localhost", 5672);

        assert!(conn.is_ok());
        let mut conn = conn.unwrap();
        let login = conn.login("/", 0, 131072, 0, "guest", "guest");
        assert!(login.is_ok());

        let channel = types::channel::Channel::new(conn, 10);

        assert!(channel.is_ok());
        let mut channel = channel.unwrap();

        let ex = channel.default_exchange();

        let reply_queue = channel.declare_queue("rpc", false, false, true, false);

        assert!(reply_queue.is_ok());

        let reply_queue = reply_queue.unwrap();

        let props = BasicProperties::null();
        let start = PreciseTime::now();
        // for i in 1..10_000_000 {
        //     // let result = rpc::rpc_call(
        //     //     &channel,
        //     //     &ex,
        //     //     &reply_queue,
        //     //     "rpc_call",
        //     //     &format!("{}", i),
        //     //     Bytes::from(format!("{}", i).as_bytes()),
        //     // ).unwrap();

        //     ex.publish(
        //         channel,
        //         "test_consumer",
        //         false,
        //         false,
        //         &props,
        //         Bytes::from(format!("{}", i).as_bytes()),
        //     );
        // }
        let futures = (1..1_000_000)
            .collect::<Vec<u64>>()
            .iter()
            .map(|i| {
                (
                    rpc::rpc_call(
                        channel,
                        &ex,
                        &reply_queue,
                        "rpc_call",
                        &format!("{}", i),
                        Bytes::from(format!("{}", i).as_bytes()),
                    ),
                    i,
                )
            })
            .map(|(future, i)| {
                let result = future.wait();
                println!("{:?}{:?}", i, result);
                result
            })
            .collect::<Vec<_>>();
        // futures.collect().wait();
        let end = start.to(PreciseTime::now());
        println!("{:?}", end);

        channel.close();
        conn.close();
    }

    #[test]
    fn consumer() {
        let config_builder: ConfiBuilder = Config::new().expect("config init");
        let config: Config = config_builder.try_into().expect("config init");
        let conn = types::connection::Connection::new("localhost", 5672);

        assert!(conn.is_ok());
        let mut conn = conn.unwrap();
        let login = conn.login("/", 0, 131072, 0, "guest", "guest");
        assert!(login.is_ok());

        let channel = types::channel::Channel::new(conn, 10);

        assert!(channel.is_ok());
        let mut channel = channel.unwrap();
        let ex = Arc::new(channel.default_exchange());
        let consumer = Consumer::new(config, 12, ex, vec!["test".to_owned()], "test_consumer");
        consumer.start();
    }
}
