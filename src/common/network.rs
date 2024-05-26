use serde::{Deserialize, Serialize};

use crate::catan::element::{GameAct, GameMsg};

use super::player::GamePlayerAction;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ServerMsg {
    Catan(GameMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ClientMsg {
    Catan(GameAct),
}

pub type NetworkServer = bevy_simplenet::Server<NetworkChannel>;
pub type NetworkServerEvent = bevy_simplenet::ServerEventFrom<NetworkChannel>;
pub type NetworkClient = bevy_simplenet::Client<NetworkChannel>;
pub type NetworkClientEvent = bevy_simplenet::ClientEventFrom<NetworkChannel>;

#[derive(Debug, Clone)]
pub struct NetworkChannel;
impl bevy_simplenet::ChannelPack for NetworkChannel {
    type ConnectMsg = ();
    type ClientMsg = ClientMsg;
    type ClientRequest = ();
    type ServerMsg = ServerMsg;
    type ServerResponse = ();
}

pub fn new_server() -> NetworkServer {
    bevy_simplenet::ServerFactory::<NetworkChannel>::new("network").new_server(
        enfync::builtin::native::TokioHandle::default(),
        "0.0.0.0:48888",
        bevy_simplenet::AcceptorConfig::Default,
        bevy_simplenet::Authenticator::None,
        bevy_simplenet::ServerConfig {
            heartbeat_interval: std::time::Duration::from_secs(300), //slower than client to avoid redundant pings
            ..Default::default()
        },
    )
}

pub fn new_client() -> NetworkClient {
    bevy_simplenet::ClientFactory::<NetworkChannel>::new("network").new_client(
        enfync::builtin::Handle::default(), //automatically selects native/WASM runtime
        url::Url::parse("ws://192.168.1.74:48888/ws").unwrap(),
        bevy_simplenet::AuthRequest::None {
            client_id: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
        },
        bevy_simplenet::ClientConfig {
            reconnect_on_disconnect: true,
            reconnect_on_server_close: true,
            heartbeat_interval: std::time::Duration::from_secs(300),
            ..Default::default()
        },
        (),
    )
}
