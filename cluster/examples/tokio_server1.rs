
use std::time::Duration;

use futures::pin_mut;
use rand::rngs::mock;
use tokio::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::macros::support;
use tokio::net::{TcpListener, TcpStream};
use tower_service::Service;

use tower::{BoxError, ServiceBuilder, ServiceExt};

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    let listener = TcpListener::bind("127.0.0.1:1111").await?;
    let message = " port:1111";
    loop {
        let (mut socket, _) = listener.accept().await?;

        tokio::spawn(async move {
            let mut buf = vec![0; 1024];

            loop {
                match socket.read(&mut buf).await {
                    // Return value of `Ok(0)` signifies that the remote has
                    // closed
                    Ok(0) => return,
                    Ok(n) => {
                        // Copy the data back to socket
                        let mut ret: Vec<u8> =Vec::from(&buf[..n]);
                        ret.append(&mut Vec::from(message.as_bytes()));
                        if socket.write_all(&ret.as_slice()).await.is_err() {
                            // Unexpected socket error. There isn't much we can
                            // do here so just stop processing.
                            return;
                        }
                    }
                    Err(_) => {
                        // Unexpected socket error. There isn't much we can do
                        // here so just stop processing.
                        println!("{}","Unexpected socket error. There isn't much we can do here so just stop processing.");
                        return;
                    }
                }
            }
        });
    }
}