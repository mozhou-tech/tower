use std::fmt;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use async_bincode::AsyncBincodeStream;
use futures_util::future::poll_fn;
use slab::Slab;
use tokio::net::{TcpListener, TcpStream};
use tokio_tower::multiplex::{Client, MultiplexTransport, Server, TagStore};
use tower_service::Service;
use serde::Deserialize;
use serde::Serialize;
use tower::{BoxError, ServiceBuilder, ServiceExt};
use tower::timeout::Timeout;
use futures_util::{future::Ready, pin_mut};
use tower_test::assert_request_eq;

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

pub struct PanicError;

impl<E> From<E> for PanicError
    where
        E: fmt::Debug,
{
    fn from(e: E) -> Self {
        panic!("{:?}", e)
    }
}

pub struct EchoService;

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

pub async fn ready<S: Service<Request>, Request>(svc: &mut S) -> Result<(), S::Error> {
    poll_fn(|cx| svc.poll_ready(cx)).await
}

#[tokio::main]
async fn main()->Result<(),BoxError> {
    let rx = TcpListener::bind("127.0.0.1:1110").await.unwrap();
    let addr = rx.local_addr().unwrap();

    // connect
    let tx = TcpStream::connect(&addr).await.unwrap();
    let tx = AsyncBincodeStream::from(tx).for_async();
    let mut tx: Client<_, PanicError, _> =
        Client::builder(MultiplexTransport::new(tx, SlabStore(Slab::new()))).build();

    let mut client = ServiceBuilder::new()
        .rate_limit(1, Duration::from_secs(1))
        .timeout(Duration::from_secs(1))
        .service(tx);
    let fut = client.ready().await.unwrap().call("hello");
    assert_request_eq!(handle, true).send_response("world");
    assert!(fut.await.unwrap());
}