use hello_world::greeter_client::GreeterClient;
use hello_world::{ChatRequest, HelloRequest};
use tonic::transport::Channel;
use tonic::Request;

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = GreeterClient::connect("http://[::1]:50051").await?;

    let request = tonic::Request::new(HelloRequest {
        name: "Tonic".into(),
    });

    let response = client.say_hello(request).await?;

    println!("RESPONSE={:?}", response);

    chat(&mut client).await?;

    Ok(())
}

async fn chat(client: &mut GreeterClient<Channel>) -> Result<(), Box<dyn std::error::Error>> {
    let outbound = async_stream::stream! {
        for i in 0..10 {
            yield ChatRequest {
                message: format!("Hello {}", i),
            };
        }
    };

    let response = client.chat(Request::new(outbound)).await?;
    let mut inbound = response.into_inner();

    while let Some(chat) = inbound.message().await? {
        println!("CHAT={:?}", chat);
    }

    Ok(())
}
