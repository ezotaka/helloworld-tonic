use std::pin::Pin;

use tonic::{transport::Server, Request, Response, Status};

use tokio_stream::{Stream, StreamExt};

use hello_world::{
    greeter_server::{Greeter, GreeterServer},
    ChatReply, ChatRequest, HelloReply, HelloRequest,
};

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

#[derive(Debug, Default)]
pub struct MyGreeter {}

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
        println!("Chat");

        let mut stream = request.into_inner();

        let output = async_stream::try_stream! {
            while let Some(chat) = stream.next().await {
                let chat = chat?;
                let reply = ChatReply {
                    message: format!("Reply: {}", chat.message),
                };
                yield reply;
            }
        };
        Ok(Response::new(Box::pin(output) as Self::ChatStream))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;
    let greeter = MyGreeter::default();

    Server::builder()
        .add_service(GreeterServer::new(greeter))
        .serve(addr)
        .await?;
    Ok(())
}
