use anyhow::{Context, Result, bail};
use bluer::{
    Address, AddressType,
    l2cap::{SeqPacket, Socket, SocketAddr},
};
use log::{debug, info};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

const AACP_PSM: u16 = 0x1001;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const IO_TIMEOUT: Duration = Duration::from_secs(3);
const CONNECTION_POLL_INTERVAL: Duration = Duration::from_millis(100);
const SEND_RETRY_LIMIT: usize = 10;

const AACP_HANDSHAKE: [u8; 16] = [
    0x00, 0x00, 0x04, 0x00, 0x01, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];
const AACP_START_AUDIO: [u8; 19] = [
    0x04, 0x00, 0x04, 0x00, 0x58, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x01, 0x82, 0x00, 0x00, 0x00,
    0x04, 0x96, 0x00,
];
const AACP_STOP_AUDIO: [u8; 12] = [
    0x04, 0x00, 0x04, 0x00, 0x58, 0x00, 0x00, 0x00, 0x02, 0x00, 0x03, 0x01,
];

pub struct AacpSession {
    socket: Arc<SeqPacket>,
    started: bool,
}

impl AacpSession {
    pub async fn connect(address: Address) -> Result<Self> {
        let socket_address = SocketAddr::new(address, AddressType::BrEdr, AACP_PSM);
        let socket = Socket::new_seq_packet().context("failed to create AACP L2CAP socket")?;
        let socket = tokio::time::timeout(CONNECT_TIMEOUT, socket.connect(socket_address))
            .await
            .context("AACP L2CAP connection timed out")?
            .context("AACP L2CAP connection failed")?;

        let ready_started = Instant::now();
        loop {
            match socket.peer_addr() {
                Ok(peer) if peer.cid != 0 => break,
                Ok(_) => {}
                Err(error) if error.raw_os_error() == Some(libc::ENOTCONN) => {}
                Err(error) => return Err(error).context("failed to verify AACP L2CAP peer"),
            }
            if ready_started.elapsed() >= CONNECT_TIMEOUT {
                bail!("AACP L2CAP peer did not become ready within 10 seconds");
            }
            tokio::time::sleep(CONNECTION_POLL_INTERVAL).await;
        }

        info!("[bt] connected {address} on PSM 0x{AACP_PSM:04X}");
        Ok(Self {
            socket: Arc::new(socket),
            started: false,
        })
    }

    pub async fn initialize(&self) -> Result<()> {
        self.send(&AACP_HANDSHAKE)
            .await
            .context("AACP handshake failed")?;
        tokio::time::sleep(Duration::from_millis(100)).await;
        info!("[aacp] session initialized");
        Ok(())
    }

    pub async fn start_audio(&mut self) -> Result<()> {
        self.send(&AACP_START_AUDIO)
            .await
            .context("AACP microphone START failed")?;
        self.started = true;
        info!("[aacp] hi-res microphone START sent");
        Ok(())
    }

    pub async fn stop_audio(&mut self) -> Result<()> {
        if self.started {
            self.send(&AACP_STOP_AUDIO)
                .await
                .context("AACP microphone STOP failed")?;
            self.started = false;
            info!("[aacp] hi-res microphone STOP sent");
        }
        Ok(())
    }

    pub async fn recv(&self, buffer: &mut [u8]) -> Result<usize> {
        self.socket
            .recv(buffer)
            .await
            .context("Bluetooth receive failed")
    }

    async fn send(&self, packet: &[u8]) -> Result<()> {
        let send = async {
            let mut attempts = 0;
            loop {
                match self.socket.send(packet).await {
                    Err(error)
                        if error.raw_os_error() == Some(libc::ENOTCONN)
                            && attempts < SEND_RETRY_LIMIT =>
                    {
                        attempts += 1;
                        debug!(
                            "[aacp] socket not ready; retrying send ({attempts}/{SEND_RETRY_LIMIT})"
                        );
                        tokio::time::sleep(CONNECTION_POLL_INTERVAL).await;
                    }
                    result => break result,
                }
            }
        };
        let written = tokio::time::timeout(IO_TIMEOUT, send)
            .await
            .context("Bluetooth send timed out")?
            .context("Bluetooth send failed")?;
        if written != packet.len() {
            bail!(
                "short Bluetooth packet write: {written}/{} bytes",
                packet.len()
            );
        }
        debug!("[aacp] sent {}-byte packet", packet.len());
        Ok(())
    }
}
