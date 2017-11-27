use libc;
use std::ptr;
use config::{ConfiBuilder, Config};
use futures::sync::mpsc;
use types::connection::Connection;
use types::channel::Channel;
use types::exchange::{Exchange, ExchangeType};
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
            let raw = self.raw_ptr();
            amqp_destroy_envelope(raw);
            libc::free(raw as *mut _);
        }
    }
}

#[derive(Debug)]
pub struct Consumer {
    receiver: Option<mpsc::Receiver<Envelope>>,
    should_stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

type Fp = Box<Fn(Envelope) -> Box<Future<Item = (), Error = ()>>>;

struct Closure {
    fp: Fp,
}

impl Closure {
    fn call(&self, envelope: Envelope) -> Box<Future<Item = (), Error = ()>> {
        (*self.fp)(envelope)
    }
}

impl Consumer {
    pub fn new(
        config: Config,
        channel_id: u16,
        exchange: Arc<Exchange>,
        routing_keys: Vec<String>,
        consumer_tag: &str,
    ) -> Consumer {
        let (sender, receiver) = mpsc::channel(65536);
        let should_stop = Arc::new(AtomicBool::new(false));
        let consumer_tag = consumer_tag.to_owned();

        let poll_stop = Arc::clone(&should_stop);
        let handle = thread::Builder::new()
            .name("consumer_poll".to_string())
            .spawn(move || {
                let mut conn = Connection::new(&config.connection.hostname, config.connection.port)
                    .expect("open amqp connect");

                conn.login(
                    &config.login.vhost,
                    config.login.channel_max,
                    config.login.frame_max,
                    config.login.heartbeat,
                    &config.login.login,
                    &config.login.password,
                ).expect("login broker");

                let mut channel = Channel::new(conn, channel_id).expect("open amqp channel");
                let queue = channel
                    .declare_queue(&consumer_tag, false, false, false, false)
                    .expect("declare queue");
                if exchange.exchange_type == ExchangeType::Topic {
                    for key in routing_keys {
                        queue.bind(channel, &exchange, &key).expect("bind queue");
                    }
                }


                poll_loop(channel, &exchange, &queue, sender, poll_stop);
                channel.close();
                conn.close();
            })
            .expect("Failed to start polling thread");

        Consumer {
            receiver: Some(receiver),
            should_stop: should_stop,
            handle: Some(handle),
        }
    }

    pub fn start<F>(mut self, f: F)
    where
        F: Fn(Envelope) -> Box<Future<Item = (), Error = ()>> + 'static,
    {
        let mut core = Core::new().unwrap();
        let handle = core.handle();
        let closure = Closure { fp: Box::new(f) };
        let server = self.receiver.take().unwrap().for_each(move |envelope| {
            handle.spawn(closure.call(envelope));
            Ok(())
        });
        core.run(server).unwrap();
    }
}

// impl Drop for Consumer {
//     fn drop(&mut self) {
//         self.should_stop.store(true, Ordering::Relaxed);
//         self.handle.take().unwrap().join();
//     }
// }

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
            queue.name(),
            amqp_empty_bytes,
            0,
            1,
            0,
            amqp_empty_table,
        );
    }
    let frame: *mut raw_rabbitmq::amqp_frame_t = unsafe {
        libc::malloc(mem::size_of::<raw_rabbitmq::amqp_frame_t>())
            as *mut raw_rabbitmq::amqp_frame_t
    };

    while !should_stop.load(Ordering::Relaxed) {
        unsafe {
            let envelope: *mut amqp_envelope_t =
                libc::malloc(mem::size_of::<amqp_envelope_t>()) as *mut amqp_envelope_t;
            amqp_maybe_release_buffers(conn);
            let ret = amqp_consume_message(conn, envelope, ptr::null_mut(), 0);
            if amqp_response_type_enum__AMQP_RESPONSE_NORMAL != ret.reply_type {
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
                                if amqp_response_type_enum__AMQP_RESPONSE_NORMAL != ret.reply_type {
                                    return;
                                    // TODO: error handle
                                }
                                amqp_destroy_message(message);
                                libc::free(message as *mut _);
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
                amqp_destroy_envelope(envelope);
                libc::free(envelope as *mut _);
            } else {
                let _ = sender.try_send(Envelope::new(envelope));
            }
        }
    }
    unsafe {
        libc::free(frame as *mut _);
    }
}
