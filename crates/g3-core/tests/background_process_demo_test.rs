use g3_core::background_process::BackgroundProcessManager;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
use std::fs;

#[test]
fn demo_background_process_with_script() {
    // Create temp directories
    let test_dir = std::env::temp_dir().join("g3_bg_demo");
    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(&test_dir).unwrap();
    
    // Create a test script
    let script_path = test_dir.join("test.sh");
    fs::write(&script_path, r#"#!/bin/bash
echo "Starting..."
for i in 1 2 3; do
    echo "Tick $i"
    sleep 0.5
done
echo "Done!"
"#).unwrap();
    
    // Make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755)).unwrap();
    }
    
    let log_dir = test_dir.join("logs");
    let manager = BackgroundProcessManager::new(log_dir);
    
    println!("\n=== Background Process Demo ===");
    
    match manager.start("demo", "./test.sh", &test_dir) {
        Ok(info) => {
            println!("✅ Started '{}' with PID {}", info.name, info.pid);
            println!("   Log file: {:?}", info.log_file);
            
            // Wait for script to produce some output
            thread::sleep(Duration::from_millis(800));
            
            // Read logs
            let logs = fs::read_to_string(&info.log_file).unwrap_or_default();
            println!("\n📜 Logs so far:\n{}", logs);
            
            // Should still be running
            assert!(manager.is_running("demo"), "Process should still be running");
            println!("🔍 Process is running: true");
            
            // Wait for completion
            thread::sleep(Duration::from_secs(2));
            
            // Read final logs
            let final_logs = fs::read_to_string(&info.log_file).unwrap_or_default();
            println!("\n📜 Final logs:\n{}", final_logs);
            
            assert!(final_logs.contains("Done!"), "Should have completed");
        }
        Err(e) => panic!("Failed to start: {}", e),
    }
    
    // Cleanup
    let _ = fs::remove_dir_all(&test_dir);
    println!("\n✅ Demo complete!");
}
