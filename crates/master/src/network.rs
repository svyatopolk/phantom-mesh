use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::Message, WebSocketStream, MaybeTlsStream};
use tokio::net::TcpStream;
use protocol::SignalMsg;

pub struct GhostClient {
    ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    pub id: String,
}

impl GhostClient {
    pub async fn connect(relay_url: &str) -> Self {
        let (mut ws_stream, _) = connect_async(relay_url).await.expect("Failed to connect");
        
        let my_id = format!("Ghost-{}", rand::random::<u16>());
        let reg_msg = SignalMsg::Register { pub_key: my_id.clone() };
        ws_stream.send(Message::Text(serde_json::to_string(&reg_msg).unwrap().into())).await.unwrap();
        
        GhostClient { ws_stream, id: my_id }
    }

    pub async fn get_peers(&mut self) -> Vec<String> {
        let req = SignalMsg::GetPeers;
        self.ws_stream.send(Message::Text(serde_json::to_string(&req).unwrap().into())).await.unwrap();

        while let Some(Ok(Message::Text(txt))) = self.ws_stream.next().await {
            if let Ok(msg) = serde_json::from_str::<SignalMsg>(&txt) {
                if let SignalMsg::Peers { list } = msg {
                    return list.into_iter().filter(|id| id != &self.id).collect();
                }
            }
        }
        vec![]
    }

    pub async fn inject(&mut self, target: String, packet: protocol::GhostPacket) {
        let packet_json = serde_json::to_string(&packet).unwrap();
        let sig = SignalMsg::Signal { target, data: packet_json };
        self.ws_stream.send(Message::Text(serde_json::to_string(&sig).unwrap().into())).await.unwrap();
    }
}
