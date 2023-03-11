use std::marker::PhantomData;
use std::task::{Context, Poll};
use std::time::Duration;

use futures::pin_mut;
use rand::rngs::mock;
use tokio::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use net::Address;
use net::dial::DefaultMakeTransport;

use tower::{BoxError};
use tower::make::MakeConnection;

struct Transport{
    socket: TcpStream,
    _marker: PhantomData<TcpStream>
}



#[tokio::main]
async fn main() -> Result<(), BoxError> {
    let transport = DefaultMakeTransport::new();
    let mut conn = transport
        .make_connection(Address::Ip("127.0.0.1:8858".parse().unwrap()))
        .await
        .unwrap();
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