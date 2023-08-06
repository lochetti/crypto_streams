use tokio::sync::mpsc;

use tokio_tungstenite::tungstenite::Error;
use tonic::{Request, Response, Status};
use worker::Msg;

mod proto {
    tonic::include_proto!("crypto_streams");
}
mod worker;

struct Server {
    sender: mpsc::Sender<Msg>,
}

#[tonic::async_trait]
impl proto::web_server::Web for Server {
    async fn connect(&self, _request: Request<()>) -> Result<Response<()>, Status> {
        println!("Connect called!");
        self.sender.send(Msg::Connect).await.ok();
        Ok(Response::new(()))
    }
    async fn disconnect(&self, _request: Request<()>) -> Result<Response<()>, Status> {
        println!("Disconnect called!");
        self.sender.send(Msg::Disconnect).await.ok();
        Ok(Response::new(()))
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
