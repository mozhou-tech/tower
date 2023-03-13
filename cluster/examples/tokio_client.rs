use std::marker::PhantomData;
use std::task::{Context, Poll};
use std::time::Duration;

use tokio::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use tower::BoxError;

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    let socket = TcpStream::connect("127.0.0.1:1111").await?;
    let (mut rd, mut wr) = io::split(socket);

    // Write data in the background
    tokio::spawn(async move {
        loop {
            wr.write_all(b"hello\r\n").await.unwrap();
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
    });
    let mut buf = vec![0; 128];
    loop {
        let n = rd.read(&mut buf).await?;

        if n == 0 {
            break;
        }
        println!("GOT {:?}", String::from_utf8(Vec::from(&buf[..n])));
    }

    Ok(())
}