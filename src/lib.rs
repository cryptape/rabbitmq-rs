#![feature(optin_builtin_traits)]
extern crate bytes;
extern crate futures;
extern crate libc;
extern crate librabbitmq_sys as raw_rabbitmq;
#[macro_use]
extern crate log;

mod util;
mod error;
mod types;
mod rpc;
mod read_message;

#[cfg(test)]
mod tests {
    use std::time::Duration;
    use libc::c_char;
    use futures::{stream, Stream, Future};
    use super::*;

    #[test]
    fn basic() {
        let conn =
            types::connection::Connection::new("localhost", 5672, Some(Duration::from_secs(1)));

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

        let props = types::props::BasicProperties::new();

        let raw_props = props.raw;


        let futures =  stream::futures_ordered((1..1_000_000).collect::<Vec<u64>>().iter().map(|i| {
            rpc::rpc_call(
                &channel,
                &ex,
                &reply_queue,
                "rpc_call",
                &format!("{}", i),
                &format!("{}", i),
            ).unwrap()
        }));
        futures.collect().wait();
        // .map(|future| future.wait() ).collect::<Vec<_>>();


        // for i in 1..1_000_000 {
        //     let rpc = rpc::rpc_call(
        //         &channel,
        //         &ex,
        //         &reply_queue,
        //         "rpc_call",
        //         &format!("{}", i),
        //         &format!("{}", i),
        //     ).unwrap().wait();
        //     println!("{}-{:?}", i, rpc);
        // }


        // let status = unsafe {
        //     (*raw_props)._flags = raw_rabbitmq::AMQP_BASIC_CONTENT_TYPE_FLAG
        //         | raw_rabbitmq::AMQP_BASIC_DELIVERY_MODE_FLAG;
        //     // | raw_rabbitmq::AMQP_BASIC_REPLY_TO_FLAG
        //     // | raw_rabbitmq::AMQP_BASIC_CORRELATION_ID_FLAG;

        //     (*raw_props).content_type =
        //         raw_rabbitmq::amqp_cstring_bytes(b"text/plain\0".as_ptr() as *const c_char);
        //     (*raw_props).delivery_mode = 2;

        //     // (*raw_props).reply_to = raw_rabbitmq::amqp_bytes_malloc_dup(queue.name_t);
        //     // (*raw_props).correlation_id =
        //     //     raw_rabbitmq::amqp_cstring_bytes(b"1\0".as_ptr() as *const c_char);

        //     producer::basic::basic_publish(
        //         &channel,
        //         "amq.direct",
        //         "test",
        //         false,
        //         false,
        //         &props,
        //         "hehe",
        //     )
        // };


        // assert!(status.is_ok());
    }
}
