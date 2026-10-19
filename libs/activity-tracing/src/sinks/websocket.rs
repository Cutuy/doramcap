use crate::{BatchView, Result, Sink};
use std::{net::TcpStream, time::Duration};
/// One connected live viewer. Accept/handshake during setup, before capture.
/// Each binary frame is a self-contained Protobuf batch.
pub struct WebSocketSink {
    socket: tungstenite::WebSocket<TcpStream>,
}
impl WebSocketSink {
    pub fn accept(stream: TcpStream, timeout: Duration) -> Result<Self> {
        if timeout.is_zero() {
            return Err("WebSocket timeout must be nonzero".into());
        }
        stream.set_read_timeout(Some(timeout))?;
        stream.set_write_timeout(Some(timeout))?;
        let socket = tungstenite::accept(stream)?;
        Ok(Self { socket })
    }
}
impl Sink for WebSocketSink {
    fn name(&self) -> &str {
        "websocket"
    }
    fn write(&mut self, b: BatchView<'_>) -> Result<()> {
        self.socket
            .send(tungstenite::Message::Binary(b.bytes.to_vec().into()))?;
        Ok(())
    }
    fn finish(&mut self) -> Result<()> {
        self.socket.close(None)?;
        Ok(())
    }
}
