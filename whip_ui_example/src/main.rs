use bevy_app::App;
use bevy_log::info;
use bevy_tasks::IoTaskPool;
use whip_ui::WhipUiPlugin;
use tracing;

mod logging_test;


fn main() {
    // Check if we should enable CLI alongside GUI
    let args: Vec<String> = std::env::args().collect();
    let enable_cli = args.iter().any(|arg| arg == "--cli" || arg == "--log-viewer");
    
    // Test the advanced logging service first
    test_logging_service();
    
    // Always run GUI, optionally with CLI
    run_gui_mode(enable_cli);
}


fn run_gui_mode(enable_cli: bool) {
    info!("Starting whip_ui example...");

    // Launch CLI if requested
    let cli_control = if enable_cli {
        println!("Launching CLI log viewer alongside GUI...");
        match whip_ui::launch_cli() {
            Ok((handle, sender)) => {
                println!("CLI launched. Use /quit to exit CLI (GUI will continue running).");
                
                // Generate some logs periodically for demonstration
                let _log_handle = std::thread::spawn(|| {
                    let mut counter = 0;
                    loop {
                        std::thread::sleep(std::time::Duration::from_secs(3));
                        counter += 1;
                        
                        tracing::info!(
                            target: "gui_demo",
                            counter = counter,
                            "GUI demo log entry #{}", counter
                        );
                        
                        if counter % 5 == 0 {
                            tracing::warn!(
                                target: "gui_demo",
                                "Periodic warning message #{}", counter / 5
                            );
                        }
                        
                        if counter % 10 == 0 {
                            tracing::error!(
                                target: "gui_demo",
                                "Simulated error message #{}", counter / 10
                            );
                        }
                    }
                });
                
                Some((handle, sender))
            }
            Err(e) => {
                eprintln!("Failed to launch CLI: {}", e);
                None
            }
        }
    } else {
        None
    };

    // Initialize IoTaskPool manually
    IoTaskPool::get_or_init(|| {
        bevy_tasks::TaskPool::new()
    });

    // Build Bevy App with WhipUiPlugin - all framework setup is abstracted away
    // Note: We disable Bevy's LogPlugin since we already set up tracing in our logging test
    App::new()
        .add_plugins(WhipUiPlugin::new("ui/layouts/main.json"))
        .run();
        
    // Clean up CLI when GUI exits
    if let Some((handle, sender)) = cli_control {
        println!("GUI exiting, shutting down CLI...");
        
        // Send shutdown signal to CLI
        if let Err(e) = sender.send(whip_ui::CliThreadCommand::Shutdown) {
            eprintln!("Failed to send shutdown signal to CLI: {}", e);
        }
        
        // Wait for CLI thread to exit cleanly
        if let Err(e) = handle.join() {
            eprintln!("CLI thread did not exit cleanly: {:?}", e);
        } else {
            println!("CLI shut down cleanly.");
        }
    }
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

