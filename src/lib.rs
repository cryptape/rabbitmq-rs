extern crate bytes;
extern crate futures;
extern crate libc;
extern crate librabbitmq_sys as raw_rabbitmq;
#[macro_use]
extern crate log;
extern crate time;

mod util;
mod error;
mod types;
mod rpc;
mod rpc_message;

#[cfg(test)]
mod tests {
    use std::time::Duration;
    use libc::c_char;
    use futures::{stream, Future, Stream};
    use super::*;
    use time::PreciseTime;
    use bytes::Bytes;

    #[test]
    fn basic() {
        let conn = types::connection::Connection::new("localhost", 5672, None);

        assert!(conn.is_ok());
        let conn = conn.unwrap();
        let login = conn.login("/", 0, 131072, 0, "guest", "guest");
        assert!(login.is_ok());

        let channel = types::channel::Channel::new(&conn, 10);

        assert!(channel.is_ok());
        let channel = channel.unwrap();

        let ex = channel.default_exchange();

        let reply_queue = channel.declare_queue("rpc".to_owned(), false, false, true, false);

        assert!(reply_queue.is_ok());

        let reply_queue = reply_queue.unwrap();
        let start = PreciseTime::now();
        // for i in 1..10000000 {
        //     let result = rpc::rpc_call(
        //         &channel,
        //         &ex,
        //         &reply_queue,
        //         "rpc_call",
        //         &format!("{}", i),
        //         Bytes::from(format!("{}", i).as_bytes()),
        //     ).unwrap();

        //     // ex.publish(&channel, "rpc_call", false, false, &props,
        //      Bytes::from(format!("{}", i).as_bytes()));
        // }
        let futures = (1..1000000)
            .collect::<Vec<u64>>()
            .iter()
            .map(|i| {
                (
                    rpc::rpc_call(
                        &channel,
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
    }
}
