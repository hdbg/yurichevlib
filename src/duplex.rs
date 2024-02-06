use tokio::sync::mpsc::{error::SendError, unbounded_channel, UnboundedReceiver, UnboundedSender};
pub struct DuplexChannel<T, Y> {
    tx: UnboundedSender<T>,
    rx: UnboundedReceiver<Y>,
}

impl<T, Y> DuplexChannel<T, Y> {
    pub fn new() -> (DuplexChannel<T, Y>, DuplexChannel<Y, T>) {
        let (tx1, rx1) = unbounded_channel();
        let (tx2, rx2) = unbounded_channel();
        (
            DuplexChannel { tx: tx2, rx: rx1 },
            DuplexChannel { tx: tx1, rx: rx2 },
        )
    }

    pub fn send(&mut self, msg: T) -> Result<(), SendError<T>> {
        self.tx.send(msg)
    }

    pub async fn recv(&mut self) -> Option<Y> {
        self.rx.recv().await
    }

    pub async fn recv_many(&mut self, items: usize) -> Vec<Y> {
        let mut buffer = Vec::with_capacity(items);
        self.rx.recv_many(&mut buffer, items).await;
        buffer
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn should_send_and_receive() {
        let (mut a, mut b) = super::DuplexChannel::new();
        a.send(42).unwrap();
        b.send(43).unwrap();
        assert_eq!(a.recv().await, Some(43));
        assert_eq!(b.recv().await, Some(42));
    }
}