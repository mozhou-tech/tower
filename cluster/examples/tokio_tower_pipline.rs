use std::fmt::Debug;
use std::pin::Pin;
use std::task::{Context, Poll};
use futures_util::future::{poll_fn, Ready, ready};
use futures_util::never::Never;
use tokio::sync::mpsc;
use tokio_tower::pipeline;

type StdError = Box<dyn std::error::Error + Send + Sync + 'static>;

/// A transport implemented using a pair of `mpsc` channels.
///
/// `mpsc::Sender` and `mpsc::Receiver` are both unidirectional. So, if we want to use `mpsc`
/// to send requests and responses between a client and server, we need *two* channels, one
/// that lets requests flow from the client to the server, and one that lets responses flow the
/// other way.
///
/// In this echo server example, requests and responses are both of type `T`, but for "real"
/// services, the two types are usually different.
struct ChannelTransport<T> {
    rcv: mpsc::UnboundedReceiver<T>,
    snd: mpsc::UnboundedSender<T>,
}

impl<T: Debug> futures_sink::Sink<T> for ChannelTransport<T> {
    type Error = StdError;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(mut self: Pin<&mut Self>, item: T) -> Result<(), Self::Error> {
        // use map_err because `T` contained in `mpsc::SendError` may not be `Send + Sync`.
        self.snd.send(item).map_err(|e| e.to_string())?;
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(())) // no-op because all sends succeed immediately
    }

    fn poll_close( self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(())) // no-op because channel is closed on drop and flush is no-op
    }
}

impl<T> futures_util::stream::Stream for ChannelTransport<T> {
    type Item = Result<T, StdError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        self.rcv.poll_recv(cx).map(|s| s.map(Ok))
    }
}

/// A service that tokio-tower should serve over the transport.
/// This one just echoes whatever it gets.
struct Echo;

impl<T> tower_service::Service<T> for Echo {
    type Response = T;
    type Error = Never;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: T) -> Self::Future {
        ready(Ok(req))
    }
}

#[tokio::main]
async fn main() {
    let (s1, r1) = mpsc::unbounded_channel();
    let (s2, r2) = mpsc::unbounded_channel();
    let pair1 = ChannelTransport{snd: s1, rcv: r2};
    let pair2 = ChannelTransport{snd: s2, rcv: r1};

    tokio::spawn(pipeline::Server::new(pair1, Echo));
    let mut client = pipeline::Client::<_, tokio_tower::Error<_, _>, _>::new(pair2);

    use tower_service::Service;
    poll_fn(|cx| client.poll_ready(cx)).await;

    let msg = "Hello, tokio-tower";
    let resp = client.call(String::from(msg)).await.expect("client call");
    println!("{}", resp);
    assert_eq!(resp, msg);
}