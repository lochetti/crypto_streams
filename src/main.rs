use tokio::sync::mpsc::{self, Receiver};

use futures_util::{stream::BoxStream, Stream};
use tokio_tungstenite::tungstenite::Error;
use tonic::{Request, Response, Status};
use worker::Msg;

mod worker;
mod proto {
    tonic::include_proto!("crypto_streams");
}

struct Server {
    sender: mpsc::Sender<Msg>,
}

struct ReceiverStream<T> {
    receiver: Receiver<T>,
}

impl<T> Stream for ReceiverStream<T> {
    type Item = Result<T, Status>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.get_mut()
            .receiver
            .poll_recv(cx)
            .map(|a| a.map(Result::Ok))
    }
}

#[tonic::async_trait]
impl proto::web_server::Web for Server {
    type ConnectStream = BoxStream<'static, Result<proto::OrderBook, Status>>;
    async fn connect(
        &self,
        _request: Request<()>,
    ) -> Result<Response<Self::ConnectStream>, Status> {
        println!("Connect called!");
        let (sender, receiver) = mpsc::channel::<proto::OrderBook>(1);
        self.sender.send(Msg::Connect(sender)).await.ok();

        let stream = Box::pin(ReceiverStream { receiver });

        Ok(Response::new(stream))
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let (sender, receiver) = mpsc::channel::<Msg>(1);

    let server = Server { sender };
    let grpc = tonic::transport::Server::builder()
        .add_service(proto::web_server::WebServer::new(server))
        .serve("127.0.0.0:9000".parse().unwrap());

    let _ = tokio::join!(grpc, worker::do_work(receiver));

    Ok(())
}
