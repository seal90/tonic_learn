use std::pin::Pin;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::{Stream, StreamExt};
use tonic::codegen::tokio_stream;
use tonic::{transport::Server, Request, Response, Status};

use hello_world::redis_operate_service_server::{ RedisOperateService, RedisOperateServiceServer};
use hello_world::{Message, SubscribeRequest, PublishRequest, PublishResponse};


pub mod hello_world {
    tonic::include_proto!("learn.tonic.redis.sdk.proto");
}

#[derive(Default)]
pub struct MyGreeter {}

#[tonic::async_trait]
impl RedisOperateService for MyGreeter {

    type SubscribeStream = Pin<Box<dyn Stream<Item = std::result::Result<Message, Status>> + Send>>;


    /// RPC service for subscribing to a channel. This is a streaming RPC, where the server sends messages to the client.
    async fn subscribe(
        &self,
        request: tonic::Request<SubscribeRequest>,
    ) -> std::result::Result<tonic::Response<Self::SubscribeStream>, tonic::Status> {

        // creating infinite stream with requested message
        let repeat = std::iter::repeat(Message {
            channel: String::from("stream"),
            data: String::from("data"),
        });
        let mut stream = Box::pin(tokio_stream::iter(repeat).throttle(Duration::from_millis(1000)));

        // spawn and channel are required if you want handle "disconnect" functionality
        // the `out_stream` will not be polled after client disconnect
        let (tx, rx) = mpsc::channel(5);
        tokio::spawn(async move {
            while let Some(item) = stream.next().await {
                match tx.send(Result::<_, Status>::Ok(item)).await {
                    Ok(_) => {
                        // item (server response) was queued to be send to client
                    }
                    Err(_item) => {
                        // output_stream was build from rx and both are dropped
                        break;
                    }
                }
            }
            println!("\tclient disconnected");
        });

        let output_stream = ReceiverStream::new(rx);
        Ok(Response::new(
            Box::pin(output_stream) as Self::SubscribeStream
        ))
    }
    /// RPC service for publishing a message to a channel.
    async fn publish(
        &self,
        request: tonic::Request<PublishRequest>,
    ) -> std::result::Result<tonic::Response<PublishResponse>, tonic::Status> {

        
        Ok(tonic::Response::new(PublishResponse{success: true}))
    }
    
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50052".parse().unwrap();
    let greeter = MyGreeter::default();

    println!("StreamServer listening on {}", addr);

    Server::builder()
        .add_service(RedisOperateServiceServer::new(greeter))
        .serve(addr)
        .await?;

    Ok(())
}
