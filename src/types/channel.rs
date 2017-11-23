use raw_rabbitmq;
use error::Error;
use types::connection::Connection;
use types::queue::Queue;
use types::exchange::{Exchange, ExchangeType};

#[derive(Clone, Copy)]
pub struct Channel {
    pub id: u16,
    pub conn: Connection,
}

impl Channel {
    pub fn new(conn: Connection, id: u16) -> Result<Channel, Error> {
        let channel_open_t = unsafe { raw_rabbitmq::amqp_channel_open(conn.raw_ptr(), id) };
        Ok(Channel { id: id, conn: conn })
    }

    pub fn declare_queue(
        self,
        name: &str,
        passive: bool,
        durable: bool,
        exclusive: bool,
        auto_delete: bool,
    ) -> Result<Queue, Error> {
        Queue::new(self, name, passive, durable, exclusive, auto_delete)
    }

    pub fn declare_exchange(
        self,
        name: &str,
        exchange_type: ExchangeType,
        passive: bool,
        durable: bool,
        auto_delete: bool,
        internal: bool,
    ) -> Result<Exchange, Error> {
        Exchange::new(
            self,
            name,
            exchange_type,
            passive,
            durable,
            auto_delete,
            internal,
        )
    }

    pub fn default_exchange(self) -> Exchange {
        Exchange::default()
    }

    pub fn close(&mut self) {
        unsafe {
            raw_rabbitmq::amqp_channel_close(
                self.conn.raw_ptr(),
                self.id,
                raw_rabbitmq::AMQP_REPLY_SUCCESS as i32,
            );
        }
    }
}
