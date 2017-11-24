use libc;
use std::ptr;
use config::Config;
use futures::sync::mpsc;
use types::connection::Connection;
use types::channel::Channel;
use types::exchange::Exchange;
use types::queue::Queue;
use util::decode_raw_bytes;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::thread::{self, JoinHandle};
use std::mem;
use futures::Stream;
use futures::{future, Future};
use tokio_core::reactor::Core;

use raw_rabbitmq::{self, amqp_basic_consume, amqp_consume_message, amqp_destroy_envelope,
                   amqp_destroy_message, amqp_empty_bytes, amqp_empty_table, amqp_envelope_t,
                   amqp_maybe_release_buffers, amqp_message_t, amqp_read_message,
                   amqp_simple_wait_frame,
                   amqp_response_type_enum__AMQP_RESPONSE_LIBRARY_EXCEPTION,
                   amqp_response_type_enum__AMQP_RESPONSE_NORMAL,
                   amqp_status_enum__AMQP_STATUS_OK,
                   amqp_status_enum__AMQP_STATUS_UNEXPECTED_STATE, AMQP_FRAME_METHOD};


const AMQP_BASIC_ACK_METHOD: u32 = 0x003C_0050;
const AMQP_BASIC_RETURN_METHOD: u32 = 0x003C_0032;
const AMQP_CHANNEL_CLOSE_METHOD: u32 = 0x0014_0028;
const AMQP_CONNECTION_CLOSE_METHOD: u32 = 0x000A_0032;

#[derive(Debug)]
pub struct Envelope {
    inner: AtomicPtr<amqp_envelope_t>,
}

impl Envelope {
    pub fn new(ptr: *mut amqp_envelope_t) -> Envelope {
        Envelope {
            inner: AtomicPtr::new(ptr),
        }
    }

    pub fn load(&self) -> amqp_envelope_t {
        unsafe { (*self.raw_ptr()) }
    }

    pub fn raw_ptr(&self) -> *mut amqp_envelope_t {
        self.inner.load(Ordering::Relaxed)
    }
}

impl Drop for Envelope {
    fn drop(&mut self) {
        unsafe {
            amqp_destroy_envelope(self.raw_ptr());
        }
    }
}

#[derive(Debug)]
pub struct Consumer {
    receiver: mpsc::Receiver<Envelope>,
    should_stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl Consumer {
    pub fn new() -> Consumer {
        let config: Config = Config::new().unwrap();
        let (sender, receiver) = mpsc::channel(65_535);
        let should_stop = Arc::new(AtomicBool::new(false));

        let poll_stop = Arc::clone(&should_stop);
        let handle = thread::Builder::new()
            .name("consumer_poll".to_string())
            .spawn(move || {
                let conn = Connection::new("localhost", 5672);
                assert!(conn.is_ok());
                let mut conn = conn.unwrap();
                let login = conn.login("/", 0, 131072, 0, "guest", "guest");
                assert!(login.is_ok());
                let channel = Channel::new(conn, 1);
                assert!(channel.is_ok());
                let mut channel = channel.unwrap();
                let ex = channel.default_exchange();
                let queue = channel
                    .declare_queue("rpc2", false, false, true, false)
                    .unwrap();
                poll_loop(channel, &ex, &queue, sender, poll_stop);
                channel.close();
                conn.close()
            })
            .expect("Failed to start polling thread");

        Consumer {
            receiver: receiver,
            should_stop: should_stop,
            handle: Some(handle),
        }
    }

    pub fn start(self) {
        let mut core = Core::new().unwrap();
        let handle = core.handle();
        let server = self.receiver.for_each(|envelope| {
            handle.spawn(process(envelope));
            Ok(())
        });
        core.run(server).unwrap();
    }
}

fn process(envelope: Envelope) -> Box<Future<Item = (), Error = ()>> {
    Box::new(future::lazy(move || {
        println!("{:?}", decode_raw_bytes(envelope.load().message.body));
        future::ok(())
    }))
}


fn poll_loop(
    channel: Channel,
    exchange: &Exchange,
    queue: &Queue,
    mut sender: mpsc::Sender<Envelope>,
    should_stop: Arc<AtomicBool>,
) {
    let conn = channel.conn.raw_ptr();
    let channel_id = channel.id;
    unsafe {
        amqp_basic_consume(
            conn,
            channel_id,
            queue.name_t,
            amqp_empty_bytes,
            0,
            1,
            0,
            amqp_empty_table,
        );
        amqp_maybe_release_buffers(conn);
    }
    let frame: *mut raw_rabbitmq::amqp_frame_t = unsafe {
        libc::malloc(mem::size_of::<raw_rabbitmq::amqp_frame_t>())
            as *mut raw_rabbitmq::amqp_frame_t
    };

    while !should_stop.load(Ordering::Relaxed) {
        unsafe {
            let envelope: *mut amqp_envelope_t =
                libc::malloc(mem::size_of::<amqp_envelope_t>()) as *mut amqp_envelope_t;

            let ret = amqp_consume_message(conn, envelope, ptr::null_mut(), 0);
            if (amqp_response_type_enum__AMQP_RESPONSE_NORMAL != ret.reply_type) {
                if amqp_response_type_enum__AMQP_RESPONSE_LIBRARY_EXCEPTION == ret.reply_type
                    && amqp_status_enum__AMQP_STATUS_UNEXPECTED_STATE == ret.library_error
                {
                    if amqp_status_enum__AMQP_STATUS_OK != amqp_simple_wait_frame(conn, frame) {
                        // TODO: error handle
                        return;
                    }
                    if AMQP_FRAME_METHOD == u32::from((*frame).frame_type) {
                        match (*frame).payload.method.id {
                            // if we've turned publisher confirms on, and we've published a message
                            // here is a message being confirmed
                            AMQP_BASIC_ACK_METHOD => {}
                            // if a published message couldn't be routed and
                            // the mandatory flag was set
                            // this is what would be returned. The message then needs to be read.
                            AMQP_BASIC_RETURN_METHOD => {
                                let message: *mut amqp_message_t =
                                    libc::malloc(mem::size_of::<amqp_message_t>())
                                        as *mut amqp_message_t;
                                let ret = amqp_read_message(conn, (*frame).channel, message, 0);
                                if (amqp_response_type_enum__AMQP_RESPONSE_NORMAL != ret.reply_type)
                                {
                                    return;
                                    // TODO: error handle
                                }
                                amqp_destroy_message(message);
                            }

                            AMQP_CHANNEL_CLOSE_METHOD | AMQP_CONNECTION_CLOSE_METHOD => {
                                return;
                            }
                            _ => {
                                println!(
                                    "unexpected method was received {:?}",
                                    (*frame).payload.method.id
                                );
                            }
                        }
                    }
                }
            } else {
                let _ = sender.try_send(Envelope::new(envelope));
            }
        }
    }
}
