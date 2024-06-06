use std::{net::SocketAddr, pin::Pin};

use tokio::sync::broadcast::{self, Sender};
use tonic::{
    transport::{Identity, Server, ServerTlsConfig},
    Request, Response, Status,
};

use tokio_stream::{Stream, StreamExt};

use hello_world::{
    greeter_server::{Greeter, GreeterServer},
    ChatReply, ChatRequest, DirectMailReply, DirectMailRequest, HelloReply, HelloRequest,
};

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

#[derive(Debug)]
pub struct MyGreeter {
    tx: Sender<ChatRequest>,
}

impl MyGreeter {
    fn new() -> Self {
        let (tx, _) = broadcast::channel(100);
        Self { tx }
    }
}

#[tonic::async_trait]
impl Greeter for MyGreeter {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloReply>, Status> {
        println!("Got a request: {:?}", request);

        let replay = hello_world::HelloReply {
            message: format!("Hello {}!", request.into_inner().name).into(),
        };

        Ok(Response::new(replay))
    }

    type ChatStream = Pin<Box<dyn Stream<Item = Result<ChatReply, Status>> + Send + 'static>>;

    async fn chat(
        &self,
        request: tonic::Request<tonic::Streaming<ChatRequest>>,
    ) -> Result<tonic::Response<Self::ChatStream>, tonic::Status> {
        let metadata = request.metadata();
        let name = metadata
            .get("name")
            .map(|val| val.to_str().unwrap_or("unknown"))
            .unwrap_or("unknown")
            .to_string();
        println!("{} has connected", name);

        let mut stream = request.into_inner();

        let tx = self.tx.clone();
        let name_clone = name.clone();
        tokio::spawn(async move {
            while let Some(Ok(chat)) = stream.next().await {
                if tx.send(chat).is_err() {
                    eprintln!("Failed to send message to channel");
                }
            }
            println!("{} is out", name_clone);
        });

        let name_clone = name.clone();
        let tx = self.tx.clone();
        let output = async_stream::try_stream! {
            let mut rx = tx.subscribe();
            loop {
                if let Ok(chat) = rx.recv().await {
                    println!("{}: {}", name_clone, chat.message);
                    let reply = ChatReply {
                        message: format!("Reply: {}", chat.message),
                    };
                    yield reply;
                }
            }
        };
        Ok(Response::new(Box::pin(output) as Self::ChatStream))
    }

    async fn direct_mail(
        &self,
        request: tonic::Request<DirectMailRequest>,
    ) -> std::result::Result<tonic::Response<DirectMailReply>, tonic::Status> {
        println!("Got a direct mail request: {:?}", request);

        let tx_clone = self.tx.clone();
        let req = request.into_inner();
        let chat = ChatRequest {
            name: req.name,
            message: req.message,
        };
        let _ = tx_clone.send(chat);

        let reply = DirectMailReply {
            message: "Hello".to_string(),
        };

        Ok(Response::new(reply))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = std::path::PathBuf::from_iter([std::env!("CARGO_MANIFEST_DIR"), "data"]);
    let cert = std::fs::read_to_string(data_dir.join("tls/server.pem"))?;
    let key = std::fs::read_to_string(data_dir.join("tls/server.key"))?;

    let identity = Identity::from_pem(cert, key);

    let local_ip = helloworld_tonic::get_local_ip().await?;
    let addr = SocketAddr::new(local_ip, 50051);
    let greeter = MyGreeter::new();

    Server::builder()
        .tls_config(ServerTlsConfig::new().identity(identity))?
        .add_service(GreeterServer::new(greeter))
        .serve(addr)
        .await?;
    Ok(())
}
