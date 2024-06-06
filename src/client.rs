use std::io::Write;
use std::net::SocketAddr;

use hello_world::greeter_client::GreeterClient;
use hello_world::{ChatRequest, DirectMailRequest};
use tokio::io::{self, AsyncBufReadExt};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::metadata::MetadataValue;
use tonic::transport::{Certificate, Channel, ClientTlsConfig};

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = std::path::PathBuf::from_iter([std::env!("CARGO_MANIFEST_DIR"), "data"]);
    let pem = std::fs::read_to_string(data_dir.join("tls/ca.pem"))?;
    let ca = Certificate::from_pem(pem);

    let tls = ClientTlsConfig::new()
        .ca_certificate(ca)
        .domain_name("example.com");

    let local_ip = helloworld_tonic::get_local_ip().await?;
    let addr = format!("https://{}:50051", local_ip);

    let channel = Channel::from_shared(addr)?
        .tls_config(tls)?
        .connect()
        .await?;

    let mut client = GreeterClient::new(channel);

    // let request = tonic::Request::new(HelloRequest {
    //     name: "Tonic".into(),
    // });

    // let response = client.say_hello(request).await?;

    // println!("RESPONSE={:?}", response);

    chat(&mut client).await?;

    Ok(())
}

async fn chat(client: &mut GreeterClient<Channel>) -> Result<(), Box<dyn std::error::Error>> {
    print!("Enter your name: ");
    std::io::stdout().flush().unwrap();

    let mut stdin = io::BufReader::new(io::stdin());
    let mut name_buf = String::new();
    stdin.read_line(&mut name_buf).await?;
    let name = name_buf.trim().to_string();
    let name_clone = name.clone();

    let (tx, rx) = mpsc::channel(100);
    tokio::spawn(async move {
        let stdin = io::stdin();
        let reader = io::BufReader::new(stdin);
        let mut lines = reader.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            if line == "quit" || line == "bye" || line == "exit" {
                break;
            } else if line == "dm" {
                let dummy_text = "test".to_string();
                let dm_req = tonic::Request::new(DirectMailRequest {
                    name: dummy_text.clone(),
                    message: dummy_text.clone(),
                });
                continue;
            }
            let req = ChatRequest {
                name: name.clone(),
                message: line,
            };
            if tx.send(req).await.is_err() {
                break;
            }
        }
    });

    let outbound = ReceiverStream::new(rx);
    let mut request = tonic::Request::new(outbound);
    if !name_clone.is_empty() {
        let meta = request.metadata_mut();
        meta.insert("name", MetadataValue::try_from(name_clone)?);
    }
    let response = client.chat(request).await?;
    let mut inbound = response.into_inner();

    let dummy_text = "test".to_string();
    let dm_req = tonic::Request::new(DirectMailRequest {
        name: dummy_text.clone(),
        message: dummy_text.clone(),
    });
    client.direct_mail(dm_req).await?;

    while let Some(chat) = inbound.message().await? {
        println!("CHAT={:?}", chat);
    }

    Ok(())
}
