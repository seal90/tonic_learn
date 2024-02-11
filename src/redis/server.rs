use std::pin::Pin;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::{Stream, StreamExt};
use tonic::codegen::tokio_stream;
use tonic::{transport::Server, Request, Response, Status};

use hello_world::redis_operate_service_server::{ RedisOperateService, RedisOperateServiceServer};
use hello_world::{Message, SubscribeRequest, PublishRequest, PublishResponse};

use redis::{Commands, PubSubCommands};

pub mod hello_world {
    tonic::include_proto!("learn.tonic.redis.sdk.proto");
}

#[derive(Default)]
pub struct MyGreeter {}

#[tonic::async_trait]
impl RedisOperateService for MyGreeter {

    // type SubscribeStream = Pin<Box<dyn tonic::codegen::tokio_stream::Stream<Item = std::result::Result<Message, tonic::Status>,>  + Send + 'static>>;
    type SubscribeStream = Pin<Box<dyn Stream<Item = std::result::Result<Message, Status>> + Send>>;

    // type SubscribeStream = ReceiverStream<std::result::Result<Message, Status>>;

    /// RPC service for subscribing to a channel. This is a streaming RPC, where the server sends messages to the client.
    async fn subscribe(
        &self,
        request: tonic::Request<SubscribeRequest>,
    ) -> std::result::Result<tonic::Response<Self::SubscribeStream>, tonic::Status> {

        let (tx, mut rx) = mpsc::channel(2);
        let channel_name = request.into_inner().channel;

        // 启动独立的tokio任务来监听Redis消息
        tokio::spawn(async move {
            let client = redis::Client::open("redis://127.0.0.1/").unwrap();
            let mut con = client.get_connection().unwrap();

            let mut pubsub = con.as_pubsub();
            pubsub.subscribe(&channel_name).unwrap();

            loop {
                let msg = pubsub.get_message().unwrap();
                let payload: String = msg.get_payload().unwrap();
                let reply = Message {
                    channel: msg.get_channel_name().to_string(),
                    data: payload.clone(),
                };
                match tx.send(Result::<_, Status>::Ok(reply)).await {
                    Ok(_) => {
                        println!("send ok")
                        // item (server response) was queued to be send to client
                    }
                    Err(_item) => {
                        // output_stream was build from rx and both are dropped
                        break;
                    }
                }
            }

        });

        tokio::spawn(async move {
            loop {
                let recv = rx.recv().await;
                println!("receive ok, data: {:?}", recv.unwrap());
            }
        });

        let (tx_1, rx_1) = mpsc::channel(2);

        // Ok(tonic::Response::new(Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx) as Self::SubscribeStream)))
        Ok(tonic::Response::new(Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx_1)) as Self::SubscribeStream))
        // Ok(tonic::Response::new(Box::pin(ReceiverStream::new(rx1)) as Self::SubscribeStream))

    }
    /// RPC service for publishing a message to a channel.
    async fn publish(
        &self,
        request: tonic::Request<PublishRequest>,
    ) -> std::result::Result<tonic::Response<PublishResponse>, tonic::Status> {

        let client = redis::Client::open("redis://127.0.0.1/").unwrap();
        let mut con = client.get_connection().unwrap();
            
        let req = request.into_inner();
        let channel = req.channel;
        let data = req.data;

        // let mut conn_lock = self.conn.lock().unwrap();
        let success = con.publish(channel, data).unwrap();
        Ok(tonic::Response::new(PublishResponse{success: success}))
    }
    
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50052".parse().unwrap();
    let greeter = MyGreeter::default();

    println!("GreeterServer listening on {}", addr);

    Server::builder()
        .add_service(RedisOperateServiceServer::new(greeter))
        .serve(addr)
        .await?;

    Ok(())
}
