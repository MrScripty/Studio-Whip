use bevy_app::App;
use bevy_log::{info, LogPlugin, Level};
use bevy_tasks::IoTaskPool;
use whip_ui::WhipUiPlugin;

mod logging_test;


fn main() {
    // Test the advanced logging service first
    test_logging_service();
    
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

/// Test the advanced logging service integration
fn test_logging_service() {
    println!("=== Testing WhipUI Advanced Logging Service ===");
    
    // Initialize the logging system
    match logging_test::init_test_logging() {
        Ok(_) => println!("✓ Logging system initialized successfully"),
        Err(e) => {
            eprintln!("✗ Failed to initialize logging: {}", e);
            return;
        }
    }
    
    // Generate test logs
    logging_test::generate_test_logs();
    
    // Wait a bit for the background worker to process logs
    std::thread::sleep(std::time::Duration::from_millis(100));
    
    // Print statistics and recent logs
    logging_test::print_log_statistics();
    
    // Test filtering
    logging_test::test_filtering();
    
    println!("=== Logging Test Complete ===\n");
}

