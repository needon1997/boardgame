use serde::{Deserialize, Serialize};

use super::catan::element::{GameAct, GameMsg};
#[cfg(not(target_family = "wasm"))]
use std::time::{Duration, Instant, SystemTime};
#[cfg(target_family = "wasm")]
use web_time::{Duration, Instant, SystemTime};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ServerMsg {
    Catan(GameMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ClientMsg {
    Catan(GameAct),
}
#[cfg(feature = "server")]
pub type NetworkServer = bevy_simplenet::Server<NetworkChannel>;
#[cfg(feature = "server")]
pub type NetworkServerEvent = bevy_simplenet::ServerEventFrom<NetworkChannel>;
#[cfg(feature = "client")]
pub type NetworkClient = bevy_simplenet::Client<NetworkChannel>;

#[cfg(feature = "client")]
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

#[cfg(feature = "server")]
pub fn new_server() -> NetworkServer {
    bevy_simplenet::ServerFactory::<NetworkChannel>::new("network").new_server(
        enfync::builtin::native::TokioHandle::default(),
        "0.0.0.0:9001",
        bevy_simplenet::AcceptorConfig::Default,
        bevy_simplenet::Authenticator::None,
        bevy_simplenet::ServerConfig {
            heartbeat_interval: std::time::Duration::from_secs(300), //slower than client to avoid redundant pings
            ..Default::default()
        },
    )
}

#[cfg(feature = "client")]
pub fn new_client() -> NetworkClient {
    bevy_simplenet::ClientFactory::<NetworkChannel>::new("network").new_client(
        enfync::builtin::Handle::default(), //automatically selects native/WASM runtime
        url::Url::parse("ws://boardgame.studio:9001/ws").unwrap(),
        bevy_simplenet::AuthRequest::None {
            client_id: SystemTime::now()
                .duration_since(
                    #[cfg(not(target_family = "wasm"))]
                    {
                        std::time::UNIX_EPOCH
                    },
                    #[cfg(target_family = "wasm")]
                    {
                        web_time::UNIX_EPOCH
                    },
                )
                .unwrap_or_default()
                .as_millis(),
        },
        bevy_simplenet::ClientConfig {
            reconnect_on_disconnect: true,
            reconnect_on_server_close: true,
            heartbeat_interval: Duration::from_secs(300),
            ..Default::default()
        },
        (),
    )
}
