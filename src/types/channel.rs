use raw_rabbitmq;
use error::Error;
use std::ffi::CString;
use types::connection::Connection;

#[derive(Debug)]
pub struct Channel<'a> {
    id: u16,
    conn: &'a Connection,
}


impl<'a> Channel<'a> {
    pub fn new(conn: &Connection, id: u16) -> Result<Channel, Error> {
        let channel_open_t = unsafe { raw_rabbitmq::amqp_channel_open(conn.ptr(), id) };
        println!("connection {:?}", channel_open_t);
        Ok(Channel { id: id, conn: conn })
    }
}

impl<'a> Drop for Channel<'a> {
    fn drop(&mut self) {
        unsafe {
            raw_rabbitmq::amqp_channel_close(
                self.conn.ptr(),
                self.id,
                raw_rabbitmq::AMQP_REPLY_SUCCESS as i32,
            );
        }
    }
}
