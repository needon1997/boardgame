use bevy::prelude::*;
use boardgame_common::network::NetworkClient;

pub(crate) struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, CameraPlugin::initialize_camera);
    }
}

impl CameraPlugin {
    fn initialize_camera(mut commands: Commands) {
        commands.spawn(Camera2dBundle::default());
    }
}

#[derive(Resource)]
pub(crate) struct Platform {
    #[cfg(target_family = "wasm")]
    pub asset_srv_addr: String,
}

impl Platform {
    pub fn load_asset(&self, path: &str) -> String {
        #[cfg(target_family = "wasm")]
        {
            format!("{}/{}", self.asset_srv_addr, path)
        }
        #[cfg(not(target_family = "wasm"))]
        {
            path.to_string()
        }
    }
}

#[derive(Resource, Debug)]
pub struct NetworkClt(NetworkClient);

impl From<NetworkClient> for NetworkClt {
    fn from(client: NetworkClient) -> Self {
        Self(client)
    }
}

impl std::ops::Deref for NetworkClt {
    type Target = NetworkClient;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for NetworkClt {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub struct WindowResizePlugin;

impl Plugin for WindowResizePlugin {
    #[cfg(target_family = "wasm")]
    fn build(&self, app: &mut App) {
        app.add_systems(Update, handle_browser_resize);
    }

    #[cfg(not(target_family = "wasm"))]
    fn build(&self, _app: &mut App) {}
}

#[cfg(target_family = "wasm")]
fn handle_browser_resize(
    mut primary_query: bevy::ecs::system::Query<
        &mut bevy::window::Window,
        bevy::ecs::query::With<bevy::window::PrimaryWindow>,
    >,
) {
    let Some(wasm_window) = web_sys::window() else {
        return;
    };
    let Ok(inner_width) = wasm_window.inner_width() else {
        return;
    };
    let Ok(inner_height) = wasm_window.inner_height() else {
        return;
    };
    let Some(target_width) = inner_width.as_f64() else {
        return;
    };
    let Some(target_height) = inner_height.as_f64() else {
        return;
    };
    for mut window in &mut primary_query {
        if window.resolution.width() != (target_width as f32)
            || window.resolution.height() != (target_height as f32)
        {
            window
                .resolution
                .set(target_width as f32, target_height as f32);
        }
    }
}
