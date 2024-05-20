use native_dialog::MessageDialog;
use native_dialog::MessageType;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::net::UdpSocket;

const MAX_MESSAGE_SIZE: usize = 1500; // UDP datagram size

pub struct ProcessMessageListener {
    socket: UdpSocket,
}

impl ProcessMessageListener {
    pub fn new() -> Option<Self> {
        // Describe local address.
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 61313);

        // Bind socket to address.
        let socket = match UdpSocket::bind(addr) {
            Ok(socket) => socket,
            Err(error) => {
                if error.kind() == std::io::ErrorKind::AddrInUse {
                    if let Some(path) = std::env::args().nth(1) {
                        if path.len() >= MAX_MESSAGE_SIZE {
                            MessageDialog::new()
                                .set_type(MessageType::Error)
                                .set_title("Error")
                                .set_text("path is too long")
                                .show_alert()
                                .unwrap();
                            return None;
                        }

                        // Send message to the listener.
                        let client = match UdpSocket::bind(SocketAddr::new(
                            IpAddr::V4(Ipv4Addr::LOCALHOST),
                            0,
                        ))
                        .and_then(|socket| socket.connect(addr).map(|()| socket))
                        {
                            Err(msg) => {
                                MessageDialog::new()
                                    .set_type(MessageType::Error)
                                    .set_title("Error")
                                    .set_text(&format!("failed to notify listener, error: {}", msg))
                                    .show_alert()
                                    .unwrap();
                                return None;
                            }
                            Ok(client) => client,
                        };

                        if let Err(msg) = client.send(path.as_bytes()) {
                            MessageDialog::new()
                                .set_type(MessageType::Error)
                                .set_title("Error")
                                .set_text(&format!("failed to notify the listener, error: {}", msg))
                                .show_alert()
                                .unwrap();
                            return None;
                        }
                    }

                    return None;
                } else {
                    MessageDialog::new()
                        .set_type(MessageType::Error)
                        .set_title("Error")
                        .set_text(&format!("error: {}", error))
                        .show_alert()
                        .unwrap();
                    return None;
                }
            }
        };

        if let Err(msg) = socket.set_nonblocking(true) {
            MessageDialog::new()
                .set_type(MessageType::Error)
                .set_title("Error")
                .set_text(&format!(
                    "failed to set non-blocking for a socket, error: {}",
                    msg
                ))
                .show_alert()
                .unwrap();
            return None;
        }

        Some(Self { socket })
    }

    pub fn process_messages(&mut self) -> Vec<String> {
        let mut buf = [0; MAX_MESSAGE_SIZE];
        let mut paths_to_process = Vec::new();

        loop {
            let _ = match self.socket.recv(&mut buf) {
                Err(_) => {
                    break;
                }
                Ok(count) => count,
            };

            let path = match std::str::from_utf8(&buf) {
                Ok(path) => path,
                Err(msg) => {
                    MessageDialog::new()
                        .set_type(MessageType::Error)
                        .set_title("Error")
                        .set_text(&format!(
                            "failed convert received message to string, error: {}",
                            msg
                        ))
                        .show_alert()
                        .unwrap();
                    panic!();
                }
            };
            let path = path.to_string();

            // Remove trailing `\0`s.
            paths_to_process.push(path.trim_matches(char::from(0)).to_string());

            // Clear buffer to receive a new message.
            buf.fill(0);
        }

        paths_to_process
    }
}
