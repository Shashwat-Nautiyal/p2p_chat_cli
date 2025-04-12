use anyhow::Result;
use iroh::protocol::Router;
use iroh::Endpoint;
use iroh_gossip::{net::Gossip, proto::TopicId};
use iroh::NodeId;
use serde::{Deserialize, Serialize};

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
        name: String::from("alice"),
    };
    // Turn the message into a `Vec`, and then use
    // `into` to coerse the `Vec` into `Bytes`
    sender.broadcast(message.to_vec().into()).await?; 

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
