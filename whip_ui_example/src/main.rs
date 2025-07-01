use bevy_app::App;
use bevy_log::info;
use bevy_tasks::IoTaskPool;
use whip_ui::WhipUiPlugin;
use tracing;

mod logging_test;


fn main() {
    // Check if we should run in CLI mode
    let args: Vec<String> = std::env::args().collect();
    let cli_mode = args.iter().any(|arg| arg == "--cli" || arg == "--log-viewer");
    
    // Test the advanced logging service first
    test_logging_service();
    
    if cli_mode {
        println!("Launching CLI log viewer...");
        run_cli_mode();
    } else {
        run_gui_mode();
    }
}

fn run_cli_mode() {
    // Launch the CLI log viewer
    match whip_ui::logging::cli::launch_cli() {
        Ok(_sender) => {
            println!("CLI launched. Use /quit to exit.");
            
            // Generate some logs periodically for demonstration
            std::thread::spawn(|| {
                let mut counter = 0;
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(3));
                    counter += 1;
                    
                    tracing::info!(
                        target: "cli_demo",
                        counter = counter,
                        "CLI demo log entry #{}", counter
                    );
                    
                    if counter % 5 == 0 {
                        tracing::warn!(
                            target: "cli_demo",
                            "Periodic warning message #{}", counter / 5
                        );
                    }
                    
                    if counter % 10 == 0 {
                        tracing::error!(
                            target: "cli_demo",
                            "Simulated error message #{}", counter / 10
                        );
                    }
                }
            });
            
            // Keep the main thread alive
            loop {
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        }
        Err(e) => {
            eprintln!("Failed to launch CLI: {}", e);
        }
    }
}

fn run_gui_mode() {
    info!("Starting whip_ui example...");

    // Initialize IoTaskPool manually
    IoTaskPool::get_or_init(|| {
        bevy_tasks::TaskPool::new()
    });

    // Build Bevy App with WhipUiPlugin - all framework setup is abstracted away
    // Note: We disable Bevy's LogPlugin since we already set up tracing in our logging test
    App::new()
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

