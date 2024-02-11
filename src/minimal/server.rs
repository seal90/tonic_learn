
use std::time::Duration;

use redis::AsyncCommands;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::{Stream, StreamExt};

#[tokio::main]
async fn main() {
    let (tx, mut rx) = mpsc::channel(2);
    let channel_name = "hello_1";

    tokio::spawn(async move {
        let client = redis::Client::open("redis://127.0.0.1/").unwrap();
        let mut con = client.get_async_connection().await.unwrap();

        let mut pubsub = con.into_pubsub();
        pubsub.subscribe(&channel_name).await.unwrap();
        let mut stream = pubsub.on_message();

        loop {
            let Some(msg) = stream.next().await else {
                println!("nothing");
                continue;
            };
            println!("received message");
            
            match tx.send("hello").await {
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
    
    let repeat = std::iter::repeat("repeat");
    let mut stream = Box::pin(tokio_stream::iter(repeat).throttle(Duration::from_millis(1000)));


    let client = redis::Client::open("redis://127.0.0.1/").unwrap();
    let mut con = client.get_async_connection().await.unwrap();
        
    let channel = "hello_1";

    
    // tokio::spawn(async move {
        while let Some(item) = stream.next().await {
            let success:  bool = con.publish(channel, item).await.unwrap();
            if success {
                println!("publish ok");
            } else {
                println!("publish failed :(");
            }
        }
        println!("\tclient disconnected");
    // });

}

