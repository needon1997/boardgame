use bevy::prelude::*;

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
