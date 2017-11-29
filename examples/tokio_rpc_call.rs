extern crate bytes;
extern crate futures;
extern crate libc;
extern crate rabbitmq;
extern crate time;
extern crate tokio_core;

use std::time::Duration;
use libc::c_char;
use time::PreciseTime;
use bytes::Bytes;

use rabbitmq::types::props::BasicProperties;
use rabbitmq::types::exchange::ExchangeType;
use rabbitmq::types::connection::Connection;
use rabbitmq::types::channel::Channel;
use rabbitmq::config::{ConfiBuilder, Config};

use rabbitmq::util::decode_raw_bytes;
use std::sync::Arc;
use rabbitmq::rpc;
use tokio_core::reactor::Core;
use futures::sync::oneshot;
use futures::stream;
use futures::Stream;
use futures::{future, Future, IntoFuture};
use rabbitmq::channel_pool::ChannelPool;


fn main() {
    let conn = Connection::new("localhost", 5672);

    assert!(conn.is_ok());
    let mut conn = conn.unwrap();
    let login = conn.login("/", 0, 131072, 0, "guest", "guest");
    assert!(login.is_ok());

    let channel = Channel::new(conn, 100000);

    assert!(channel.is_ok());
    let mut channel = channel.unwrap();

    let ex = channel.default_exchange();

    let reply_queue = channel.declare_queue("rpc_call", false, false, true, false);

    assert!(reply_queue.is_ok());

    let reply_queue = reply_queue.unwrap();


    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let calls = (1..100_000).collect::<Vec<u64>>();
    let calls = stream::iter_ok::<_, ()>(calls);

    let (stop, stopped) = oneshot::channel::<()>();

    let pool = ChannelPool::new(conn, 100).unwrap();

    let start = PreciseTime::now();
    {
        let rpc_call = calls.for_each(move |i| {
            let ch = pool.get();
            let rpc = rpc::rpc_call(
                        &ch,
                        &ex,
                        &reply_queue,
                        "rpc_server",
                        &format!("{}", i),
                        Bytes::from(format!("{}", i).as_bytes()),
                    ).map(|bytes| {
                        println!("{:?}", bytes);
                        ()
                    }).map_err(|_| ());
            handle.spawn(rpc);
            Ok(()) 
        });

        let _ = core.run(rpc_call.join(stopped.map_err(|_| ())));
    }

    let end = start.to(PreciseTime::now());
    println!("{:?}", end);

    channel.close();
    conn.close();
}
