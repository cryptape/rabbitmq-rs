use std::collections::vec_deque::VecDeque;
use types::channel::Channel;
use types::connection::Connection;
use std::rc::Rc;
use error::Error;
use std::ops::Deref;
use parking_lot::Mutex;

// simple ring channel pool

#[derive(Clone)]
pub struct ChannelPool {
    shared: Rc<SharedPool>,
}

pub struct SharedPool {
    channels: Mutex<VecDeque<Channel>>,
    capacity: usize,
}


impl ChannelPool {
    pub fn new(conn: Connection, capacity: usize) -> Result<ChannelPool, Error> {
        let shared = SharedPool {
            channels: Mutex::new(VecDeque::with_capacity(capacity)),
            capacity: capacity,
        };
        open_idle_channel(conn, &shared)?;

        Ok(ChannelPool {
            shared: Rc::new(shared),
        })
    }

    pub fn get(&self) -> PooledChannel {
        loop {
            match self.try_get() {
                Some(channel) => return channel,
                None => {}
            }
        }
    }

    fn try_get(&self) -> Option<PooledChannel> {
        let channel = {
            let mut guard = self.shared.channels.lock();
            guard.pop_front()
        };
        channel.map(|channel| {
            PooledChannel {
                pool: self.clone(),
                channel: Some(channel),
            }
        })
    }

    fn put_back(&self, channel: Channel) {
        let mut guard = self.shared.channels.lock();
        guard.push_back(channel);
    }
}

fn open_idle_channel(conn: Connection, shared: &SharedPool) -> Result<(), Error> {
    let mut guard = shared.channels.lock();
    let capacity = shared.capacity as u16 + 1;
    for id in 1..capacity {
        let channel = Channel::new(conn, id)?;
        guard.push_back(channel);
    }
    Ok(())
}


pub struct PooledChannel {
    pool: ChannelPool,
    channel: Option<Channel>,
}

impl Drop for PooledChannel {
    fn drop(&mut self) {
        self.pool.put_back(self.channel.take().unwrap());
    }
}


impl Deref for PooledChannel {
    type Target = Channel;

    fn deref(&self) -> &Channel {
        self.channel.as_ref().unwrap()
    }
}
