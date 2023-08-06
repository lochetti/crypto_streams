use futures_util::stream::StreamExt;
use futures_util::SinkExt;
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;

use tokio_tungstenite::tungstenite::{Error, Message};
use tonic::{Request, Response, Status};
use url::Url;

mod proto {
    tonic::include_proto!("crypto_streams");
}

#[derive(Debug)]
enum Msg {
    Connect,
    Disconnect,
}

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
    let tonic = tonic::transport::Server::builder()
        .add_service(proto::web_server::WebServer::new(server))
        .serve("127.0.0.0:9000".parse().unwrap());

    let _ = tokio::join!(tonic, worker(receiver));

    Ok(())
}

async fn worker(mut receiver: mpsc::Receiver<Msg>) -> Result<(), Error> {
    let mut clients: u32 = 0;

    //No connected client, yet
    while let Some(msg) = receiver.recv().await {
        match msg {
            Msg::Connect => clients += 1,
            Msg::Disconnect => {
                println!("We don't have any connected clients to disconnect :(");
                continue;
            }
        }

        let address = "wss://stream.binance.com:443/ws";
        let (mut socket, _) = connect_async(Url::parse(address).unwrap()).await?;
        println!("Connected to WebSocket stream");
        socket
            .send(Message::Text(String::from(
                "{\"method\":\"SUBSCRIBE\", \"params\": [\"btcusdt@depth20\"], \"id\": 1}",
            )))
            .await?;

        // At least one client connected.
        loop {
            tokio::select! {
                receiver_msg = receiver.recv() => {
                    match receiver_msg {
                        None => {
                            println!("Received None from GRPC request");
                            return Ok(())
                        },
                        Some(Msg::Connect) => {
                            clients +=1;
                        },
                        Some(Msg::Disconnect) => {
                            clients -=1;
                            if clients == 0 {
                                break;
                            }
                        }
                    }

                }
                socket_msg = socket.next() => {
                    match socket_msg {
                        None => {
                            println!("Received None from binance");
                            return Ok(());
                        }
                        Some(Err(e)) => {
                            println!("Received Some(Err) from binance {e:?}");
                            return Ok(());
                        }
                        Some(Ok(Message::Text(text))) => {
                            println!("Received: {}", text);
                        }
                        Some(Ok(Message::Binary(bytes))) => {
                            println!("Received: {}", String::from_utf8(bytes).unwrap());
                        }
                        Some(Ok(Message::Ping(frame))) => {
                            socket.send(Message::Pong(frame)).await?;
                            println!("Sent pong!");
                        }
                        Some(Ok(Message::Close(_))) => {
                            println!("WebSocket connection closed by the server");
                            //TODO: At this momement, we need to reconect.
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    Ok(())
}
