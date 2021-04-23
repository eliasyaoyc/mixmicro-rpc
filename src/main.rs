use lazy_static::lazy_static;
use bytes::Bytes;
use hyper::{
    body::HttpBody,
    header::{self, HeaderValue},
    Body, Request, Response, Server, StatusCode,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::net::TcpListener;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use structopt::StructOpt;
use tower::{make::Shared, ServiceBuilder};
use warp::{filters, path};
use warp::{Filter, Rejection, Reply};
use tracing::log::Level::Warn;

lazy_static!(
 static ref HASHMAP: HashMap<String, &'static str> = {
        let mut m = HashMap::new();
        m.insert("foo".into(), "foo");
        m.insert("bar".into(), "bar");
        m.insert("baz".into(), "baz");
        m
    };
);

#[derive(Debug, StructOpt)]
struct Config {
    #[structopt(long, short = "p", default_value = "3000")]
    port: u16,
}

#[derive(Clone, Debug)]
struct State {
    db: Arc<RwLock<HashMap<String, Bytes>>>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = Config::from_args();

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = TcpListener::bind(addr).unwrap();

    serve_forever(listener).await.expect("server error");
}

async fn serve_forever(listener: TcpListener) -> Result<(), hyper::Error> {
    let filter = get().or(set());

    let warp_service = warp::service(filter);

    let service = ServiceBuilder::new()
        .timeout(Duration::from_secs(10))
        .service(warp_service);

    let addr = listener.local_addr().unwrap();
    tracing::info!("Listening on {}", addr);

    Server::from_tcp(listener)
        .unwrap()
        .serve(Shared::new(service))
        .await?;
    Ok(())
}

pub fn get() -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    warp::get()
        .and(path!(String))
        .map(|path: String| {
            if let Some(value) = HASHMAP.get(&path).cloned()  {
               Response::new(Body::from(value))
            }else {
                Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::empty())
                    .unwrap()
            }
        })
}

pub fn set() -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    warp::post()
        .and(path!(String))
        .and(filters::ext::get::<State>())
        .and(filters::body::bytes())
        .map(|path: String, state: State, value: Bytes| {
            let mut state = state.db.write().unwrap();

            state.insert(path, value);
            Response::new(Body::empty())
        })
}
