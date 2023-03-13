use async_bincode::AsyncBincodeStream;
use slab::Slab;
use std::pin::Pin;
use futures_util::future::poll_fn;
use tokio::net::{TcpListener, TcpStream};
use tokio_tower::multiplex::{
    client::VecDequePendingStore, Client, MultiplexTransport, Server, TagStore,
};
use tower_service::Service;
use serde::Serialize;
use serde::Deserialize;

pub(crate) struct SlabStore(Slab<()>);

fn unwrap<T>(r: Result<T, PanicError>) -> T {
    if let Ok(t) = r {
        t
    } else {
        unreachable!();
    }
}

#[derive(Serialize, Deserialize)]
pub struct Request {
    tag: usize,
    value: u32,
}

impl Request {
    pub fn new(val: u32) -> Self {
        Request { tag: 0, value: val }
    }

    pub fn check(&self, expected: u32) {
        assert_eq!(self.value, expected);
    }
}

#[derive(Serialize, Deserialize)]
pub struct Response {
    tag: usize,
    value: u32,
}

impl From<Request> for Response {
    fn from(r: Request) -> Response {
        Response {
            tag: r.tag,
            value: r.value,
        }
    }
}

impl Response {
    pub fn check(&self, expected: u32) {
        assert_eq!(self.value, expected);
    }

    pub fn get_tag(&self) -> usize {
        self.tag
    }
}

impl Request {
    pub fn set_tag(&mut self, tag: usize) {
        self.tag = tag;
    }
}
struct PanicError;
use std::fmt;
use std::task::{Context, Poll};

impl<E> From<E> for PanicError
    where
        E: fmt::Debug,
{
    fn from(e: E) -> Self {
        panic!("{:?}", e)
    }
}
struct EchoService;
impl Service<Request> for EchoService {
    type Response = Response;
    type Error = ();
    type Future = futures_util::future::Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _: &mut Context) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, r: Request) -> Self::Future {
        futures_util::future::ok(Response::from(r))
    }
}
impl TagStore<Request, Response> for SlabStore {
    type Tag = usize;
    fn assign_tag(mut self: Pin<&mut Self>, request: &mut Request) -> usize {
        let tag = self.0.insert(());
        request.set_tag(tag);
        tag
    }
    fn finish_tag(mut self: Pin<&mut Self>, response: &Response) -> usize {
        let tag = response.get_tag();
        self.0.remove(tag);
        tag
    }
}

async fn ready<S: Service<Request>, Request>(svc: &mut S) -> Result<(), S::Error> {
    poll_fn(|cx| svc.poll_ready(cx)).await
}

#[tokio::main]
async fn main() {
    let rx = TcpListener::bind("127.0.0.1:1110").await.unwrap();
    let addr = rx.local_addr().unwrap();

    // connect
    let tx = TcpStream::connect(&addr).await.unwrap();
    let tx = AsyncBincodeStream::from(tx).for_async();
    let mut tx: Client<_, PanicError, _> =
        Client::builder(MultiplexTransport::new(tx, SlabStore(Slab::new()))).build();

    // accept
    let (rx, _) = rx.accept().await.unwrap();
    let rx = AsyncBincodeStream::from(rx).for_async();
    let server = Server::new(rx, EchoService);

    tokio::spawn(async move { server.await.unwrap() });

    unwrap(ready(&mut tx).await);
    let fut1 = tx.call(Request::new(1));
    unwrap(ready(&mut tx).await);
    let fut2 = tx.call(Request::new(2));
    unwrap(ready(&mut tx).await);
    let fut3 = tx.call(Request::new(3));
    unwrap(fut1.await).check(1);
    unwrap(fut2.await).check(2);
    unwrap(fut3.await).check(3);
}
