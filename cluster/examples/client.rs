use std::fmt;
use std::pin::Pin;

use async_bincode::AsyncBincodeStream;
use slab::Slab;

use tokio_tower::multiplex::{Client, MultiplexTransport, TagStore};
use tower::{BoxError};
use serde::Serialize;
use serde::Deserialize;
use tower_service::Service;
use net::Address;
use net::dial::DefaultMakeTransport;

struct PanicError;

impl<E> From<E> for PanicError
    where
        E: fmt::Debug,
{
    fn from(e: E) -> Self {
        panic!("{:?}", e)
    }
}
#[derive(Serialize, Deserialize)]
pub struct Request {
    tag: usize,
    value: u32,
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


impl Request {
    pub fn new(val: u32) -> Self {
        Request { tag: 0, value: val }
    }

    pub fn check(&self, expected: u32) {
        assert_eq!(self.value, expected);
    }
}
pub(crate) struct SlabStore(Slab<()>);

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


#[tokio::main]
async fn main() -> Result<(), BoxError> {
    let transport = DefaultMakeTransport::new();
    let mut conn = transport
        .make_connection(Address::Ip("127.0.0.1:1111".parse().unwrap()))
        .await
        .unwrap();
    // connect
    let tx = AsyncBincodeStream::from(conn.stream).for_async();
    let mut tx: Client<_, PanicError, _> =
        Client::builder(MultiplexTransport::new(tx, SlabStore(Slab::new()))).build();
    if tx.poll_ready().is_ready() {
        tx.call(Request::new(11));
    }
    Ok(())
}