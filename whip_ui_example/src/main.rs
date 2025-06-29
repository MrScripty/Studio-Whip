use bevy_app::App;
use bevy_log::{info, LogPlugin, Level};
use bevy_tasks::IoTaskPool;
use whip_ui::WhipUiPlugin;


fn main() {
    info!("Starting whip_ui example...");

    // Initialize IoTaskPool manually
    IoTaskPool::get_or_init(|| {
        bevy_tasks::TaskPool::new()
    });

    // Build Bevy App with WhipUiPlugin - all framework setup is abstracted away
    App::new()
        .add_plugins(LogPlugin {
            level: Level::INFO,
            filter: "wgpu=error,whip_ui=info,whip_ui::gui_framework::systems::state_tracking=debug".to_string(),
            ..Default::default()
        })
        .add_plugins(WhipUiPlugin::new("ui/layouts/main.json"))
        .run();
}

