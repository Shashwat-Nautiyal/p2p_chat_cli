use std::collections::HashMap;
use anyhow::Result;
use iroh::protocol::Router;
use iroh::Endpoint;
use iroh_gossip::{ proto::TopicId};
use iroh_gossip::net::{Event, GossipEvent, GossipReceiver, Gossip};
use iroh::NodeId;
use serde::{Deserialize, Serialize};
use futures_lite::StreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    let endpoint = Endpoint::builder().discovery_n0().bind().await?;

    println!("> our node id: {}", endpoint.node_id());
    let gossip = Gossip::builder().spawn(endpoint.clone()).await?;

    let router = Router::builder(endpoint.clone())
        .accept(iroh_gossip::ALPN, gossip.clone())
        .spawn()
        .await?;

    // Create a new topic.
    let id = TopicId::from_bytes(rand::random());
    let node_ids = vec![];

    // Subscribe to the topic.
    // Since the `node_ids` list is empty, we will
    // subscribe to the topic, but not attempt to
    // connect to any other nodes.
    let topic = gossip.subscribe(id, node_ids)?;

    // `split` splits the topic into the `GossipSender`
    // and `GossipReceiver` portions
    let (sender, _receiver) = topic.split();


    let message = Message::AboutMe {
        from: endpoint.node_id(),
        name: String::from("shash"),
    };
    // Turn the message into a `Vec`, and then use
    // `into` to coerse the `Vec` into `Bytes`
    sender.broadcast(message.to_vec().into()).await?;

    // subscribe and print loop
    tokio::spawn(subscribe_loop(_receiver));

    // create a multi-provider, single-consumer channel
    let (line_tx, mut line_rx) = tokio::sync::mpsc::channel(1);
    // and pass the `sender` portion to the `input_loop`
    std::thread::spawn(move || input_loop(line_tx));

    // broadcast each line we type
    println!("> type a message and hit enter to broadcast...");
    // listen for lines that we have typed to be sent from `stdin`
    while let Some(text) = line_rx.recv().await {
        // create a message from the text
        let message = Message::Message {
            from: endpoint.node_id(),
            text: text.clone(),
        };
        // broadcast the encoded message
        sender.broadcast(message.to_vec().into()).await?;
        // print to ourselves the text that we sent
        println!("> sent: {text}");
    }


    router.shutdown().await?;

    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
enum Message {
    AboutMe { from: NodeId, name: String },
    Message { from: NodeId, text: String },
}

impl Message {
    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        serde_json::from_slice(bytes).map_err(Into::into)
    }

    pub fn to_vec(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("serde_json::to_vec is infallible")
    }
}

async fn subscribe_loop(mut receiver: GossipReceiver) -> Result<()> {
    // keep track of the mapping between `NodeId`s and names
    let mut names = HashMap::new();
    // iterate over all events
    while let Some(event) = receiver.try_next().await? {
        // if the Event is a `GossipEvent::Received`, let's deserialize the message:
        if let Event::Gossip(GossipEvent::Received(msg)) = event {
            // deserialize the message and match on the
            // message type:
            match Message::from_bytes(&msg.content)? {
                Message::AboutMe { from, name } => {
                    // if it's an `AboutMe` message
                    // add and entry into the map
                    // and print the name
                    names.insert(from, name.clone());
                    println!("> {} is now known as {}", from.fmt_short(), name);
                }
                Message::Message { from, text } => {
                    // if it's a `Message` message,
                    // get the name from the map
                    // and print the message
                    let name = names
                        .get(&from)
                        .map_or_else(|| from.fmt_short(), String::to_string);
                    println!("{}: {}", name, text);
                }
            }
        }
    }
    Ok(())
}

fn input_loop(line_tx: tokio::sync::mpsc::Sender<String>) -> Result<()> {
    // create a new string buffer
    let mut buffer = String::new();
    // get a handle on `Stdin`
    let stdin = std::io::stdin(); // We get `Stdin` here.
    loop {
        // loop through reading from the buffer...
        stdin.read_line(&mut buffer)?;
        // and then sending over the channel
        line_tx.blocking_send(buffer.clone())?;
        // clear the buffer after we've sent the content
        buffer.clear();
    }
}