//! WebDriver browser automation tools.

use std::sync::Arc;
use anyhow::Result;
use g3_computer_control::WebDriverController;
use tracing::{debug, warn};

use crate::ui_writer::UiWriter;
use crate::webdriver_session::WebDriverSession;
use crate::ToolCall;

use super::executor::ToolContext;

// ─────────────────────────────────────────────────────────────────────────────
// Session helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Acquire the WebDriver session, returning an error message if unavailable.
async fn get_session<W: UiWriter>(
    ctx: &ToolContext<'_, W>,
) -> Result<Arc<tokio::sync::Mutex<WebDriverSession>>, String> {
    if !ctx.config.webdriver.enabled {
        return Err("❌ WebDriver is not enabled. Use --webdriver flag to enable.".to_string());
    }

    let session_guard = ctx.webdriver_session.read().await;
    match session_guard.as_ref() {
        Some(s) => Ok(s.clone()),
        None => Err("❌ No active WebDriver session. Call webdriver_start first.".to_string()),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tool implementations
// ─────────────────────────────────────────────────────────────────────────────

/// Execute the `webdriver_start` tool.
pub async fn execute_webdriver_start<W: UiWriter>(
    tool_call: &ToolCall,
    ctx: &ToolContext<'_, W>,
) -> Result<String> {
    debug!("Processing webdriver_start tool call");
    let _ = tool_call; // unused

    if !ctx.config.webdriver.enabled {
        return Ok("❌ WebDriver is not enabled. Use --webdriver flag to enable.".to_string());
    }

    // Check if session already exists
    let session_guard = ctx.webdriver_session.read().await;
    if session_guard.is_some() {
        drop(session_guard);
        return Ok("✅ WebDriver session already active".to_string());
    }
    drop(session_guard);

    // Determine which browser to use based on config
    use g3_config::WebDriverBrowser;
    match &ctx.config.webdriver.browser {
        WebDriverBrowser::Safari => start_safari_driver(ctx).await,
        WebDriverBrowser::ChromeHeadless => start_chrome_driver(ctx).await,
    }
}

async fn start_safari_driver<W: UiWriter>(ctx: &ToolContext<'_, W>) -> Result<String> {
    let port = ctx.config.webdriver.safari_port;

    let driver_result = tokio::process::Command::new("safaridriver")
        .arg("--port")
        .arg(port.to_string())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();

    let mut webdriver_process = match driver_result {
        Ok(process) => process,
        Err(e) => {
            return Ok(format!(
                "❌ Failed to start safaridriver: {}\n\nMake sure safaridriver is installed.",
                e
            ));
        }
    };

    // Wait for safaridriver to start up
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

    // Connect to SafariDriver
    match g3_computer_control::SafariDriver::with_port(port).await {
        Ok(driver) => {
            let session =
                std::sync::Arc::new(tokio::sync::Mutex::new(WebDriverSession::Safari(driver)));
            *ctx.webdriver_session.write().await = Some(session);
            *ctx.webdriver_process.write().await = Some(webdriver_process);

            Ok(
                "✅ WebDriver session started successfully! Safari should open automatically."
                    .to_string(),
            )
        }
        Err(e) => {
            let _ = webdriver_process.kill().await;
            Ok(format!(
                "❌ Failed to connect to SafariDriver: {}\n\n\
                This might be because:\n  \
                - Safari Remote Automation is not enabled (run: safaridriver --enable)\n  \
                - Port {} is already in use\n  \
                - Safari failed to start\n  \
                - Network connectivity issue\n\n\
                To enable Remote Automation:\n  \
                1. Run: safaridriver --enable (requires password, one-time setup)\n  \
                2. Or manually: Safari → Develop → Allow Remote Automation",
                e, port
            ))
        }
    }
}

async fn start_chrome_driver<W: UiWriter>(ctx: &ToolContext<'_, W>) -> Result<String> {
    let port = ctx.config.webdriver.chrome_port;

    // Start chromedriver process
    let driver_result = tokio::process::Command::new("chromedriver")
        .arg(format!("--port={}", port))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();

    let mut webdriver_process = match driver_result {
        Ok(process) => process,
        Err(e) => {
            return Ok(format!(
                "❌ Failed to start chromedriver: {}\n\n\
                Make sure chromedriver is installed and in your PATH.\n\n\
                Install with:\n  \
                - macOS: brew install chromedriver\n  \
                - Linux: apt install chromium-chromedriver\n  \
                - Or download from: https://chromedriver.chromium.org/downloads",
                e
            ));
        }
    };

    // Wait for chromedriver to be ready with retry loop
    let max_retries = 10;
    let mut last_error = None;

    for attempt in 0..max_retries {
        // Wait before each attempt (200ms between retries, total max ~2s)
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        // Try to connect to ChromeDriver in headless mode (with optional custom binary)
        let driver_result = match &ctx.config.webdriver.chrome_binary {
            Some(binary) => {
                g3_computer_control::ChromeDriver::with_port_headless_and_binary(port, Some(binary))
                    .await
            }
            None => g3_computer_control::ChromeDriver::with_port_headless(port).await,
        };

        match driver_result {
            Ok(driver) => {
                let session =
                    std::sync::Arc::new(tokio::sync::Mutex::new(WebDriverSession::Chrome(driver)));
                *ctx.webdriver_session.write().await = Some(session);
                *ctx.webdriver_process.write().await = Some(webdriver_process);

                return Ok(
                    "✅ WebDriver session started successfully! Chrome is running in headless mode (no visible window)."
                        .to_string(),
                );
            }
            Err(e) => {
                last_error = Some(e);
                if attempt < max_retries - 1 {
                    continue;
                }
            }
        }
    }

    // All retries failed
    let _ = webdriver_process.kill().await;
    let error_msg = last_error
        .map(|e| e.to_string())
        .unwrap_or_else(|| "Unknown error".to_string());
    Ok(format!(
        "❌ Failed to connect to ChromeDriver after {} attempts: {}\n\n\
        This might be because:\n  \
        - Chrome is not installed\n  \
        - ChromeDriver version doesn't match Chrome version\n  \
        - Port {} is already in use\n\n\
        Make sure Chrome and ChromeDriver are installed and compatible.",
        max_retries, error_msg, port
    ))
}

/// Execute the `webdriver_navigate` tool.
pub async fn execute_webdriver_navigate<W: UiWriter>(
    tool_call: &ToolCall,
    ctx: &ToolContext<'_, W>,
) -> Result<String> {
    debug!("Processing webdriver_navigate tool call");

    let session = match get_session(ctx).await {
        Ok(s) => s,
        Err(msg) => return Ok(msg),
    };

    let url = match tool_call.args.get("url").and_then(|v| v.as_str()) {
        Some(u) => u,
        None => return Ok("❌ Missing url argument".to_string()),
    };

    let mut driver = session.lock().await;
    match driver.navigate(url).await {
        Ok(_) => Ok(format!("✅ Navigated to {}", url)),
        Err(e) => Ok(format!("❌ Failed to navigate: {}", e)),
    }
}

/// Execute the `webdriver_get_url` tool.
pub async fn execute_webdriver_get_url<W: UiWriter>(
    tool_call: &ToolCall,
    ctx: &ToolContext<'_, W>,
) -> Result<String> {
    debug!("Processing webdriver_get_url tool call");
    let _ = tool_call; // unused

    let session = match get_session(ctx).await {
        Ok(s) => s,
        Err(msg) => return Ok(msg),
    };

    let driver = session.lock().await;
    match driver.current_url().await {
        Ok(url) => Ok(format!("Current URL: {}", url)),
        Err(e) => Ok(format!("❌ Failed to get URL: {}", e)),
    }
}

/// Execute the `webdriver_get_title` tool.
pub async fn execute_webdriver_get_title<W: UiWriter>(
    tool_call: &ToolCall,
    ctx: &ToolContext<'_, W>,
) -> Result<String> {
    debug!("Processing webdriver_get_title tool call");
    let _ = tool_call; // unused

    let session = match get_session(ctx).await {
        Ok(s) => s,
        Err(msg) => return Ok(msg),
    };

    let driver = session.lock().await;
    match driver.title().await {
        Ok(title) => Ok(format!("Page title: {}", title)),
        Err(e) => Ok(format!("❌ Failed to get title: {}", e)),
    }
}

/// Execute the `webdriver_find_element` tool.
pub async fn execute_webdriver_find_element<W: UiWriter>(
    tool_call: &ToolCall,
    ctx: &ToolContext<'_, W>,
) -> Result<String> {
    debug!("Processing webdriver_find_element tool call");

    let session = match get_session(ctx).await {
        Ok(s) => s,
        Err(msg) => return Ok(msg),
    };

    let selector = match tool_call.args.get("selector").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return Ok("❌ Missing selector argument".to_string()),
    };

    let mut driver = session.lock().await;
    match driver.find_element(selector).await {
        Ok(elem) => match elem.text().await {
            Ok(text) => Ok(format!("Element text: {}", text)),
            Err(e) => Ok(format!("❌ Failed to get element text: {}", e)),
        },
        Err(e) => Ok(format!("❌ Failed to find element '{}': {}", selector, e)),
    }
}

/// Execute the `webdriver_find_elements` tool.
pub async fn execute_webdriver_find_elements<W: UiWriter>(
    tool_call: &ToolCall,
    ctx: &ToolContext<'_, W>,
) -> Result<String> {
    debug!("Processing webdriver_find_elements tool call");

    let session = match get_session(ctx).await {
        Ok(s) => s,
        Err(msg) => return Ok(msg),
    };

    let selector = match tool_call.args.get("selector").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return Ok("❌ Missing selector argument".to_string()),
    };

    let mut driver = session.lock().await;
    match driver.find_elements(selector).await {
        Ok(elements) => {
            let mut results = Vec::new();
            for (i, elem) in elements.iter().enumerate() {
                match elem.text().await {
                    Ok(text) => results.push(format!("[{}]: {}", i, text)),
                    Err(_) => results.push(format!("[{}]: <error getting text>", i)),
                }
            }
            Ok(format!(
                "Found {} elements:\n{}",
                results.len(),
                results.join("\n")
            ))
        }
        Err(e) => Ok(format!("❌ Failed to find elements '{}': {}", selector, e)),
    }
}

/// Execute the `webdriver_click` tool.
pub async fn execute_webdriver_click<W: UiWriter>(
    tool_call: &ToolCall,
    ctx: &ToolContext<'_, W>,
) -> Result<String> {
    debug!("Processing webdriver_click tool call");

    let session = match get_session(ctx).await {
        Ok(s) => s,
        Err(msg) => return Ok(msg),
    };

    let selector = match tool_call.args.get("selector").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return Ok("❌ Missing selector argument".to_string()),
    };

    let mut driver = session.lock().await;
    match driver.find_element(selector).await {
        Ok(mut elem) => match elem.click().await {
            Ok(_) => Ok(format!("✅ Clicked element '{}'", selector)),
            Err(e) => Ok(format!("❌ Failed to click element: {}", e)),
        },
        Err(e) => Ok(format!("❌ Failed to find element '{}': {}", selector, e)),
    }
}

/// Execute the `webdriver_send_keys` tool.
pub async fn execute_webdriver_send_keys<W: UiWriter>(
    tool_call: &ToolCall,
    ctx: &ToolContext<'_, W>,
) -> Result<String> {
    debug!("Processing webdriver_send_keys tool call");

    let session = match get_session(ctx).await {
        Ok(s) => s,
        Err(msg) => return Ok(msg),
    };

    let selector = match tool_call.args.get("selector").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return Ok("❌ Missing selector argument".to_string()),
    };

    let text = match tool_call.args.get("text").and_then(|v| v.as_str()) {
        Some(t) => t,
        None => return Ok("❌ Missing text argument".to_string()),
    };

    let clear_first = tool_call
        .args
        .get("clear_first")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let mut driver = session.lock().await;
    match driver.find_element(selector).await {
        Ok(mut elem) => {
            if clear_first {
                if let Err(e) = elem.clear().await {
                    return Ok(format!("❌ Failed to clear element: {}", e));
                }
            }
            match elem.send_keys(text).await {
                Ok(_) => Ok(format!("✅ Sent keys to element '{}'", selector)),
                Err(e) => Ok(format!("❌ Failed to send keys: {}", e)),
            }
        }
        Err(e) => Ok(format!("❌ Failed to find element '{}': {}", selector, e)),
    }
}

/// Execute the `webdriver_execute_script` tool.
pub async fn execute_webdriver_execute_script<W: UiWriter>(
    tool_call: &ToolCall,
    ctx: &ToolContext<'_, W>,
) -> Result<String> {
    debug!("Processing webdriver_execute_script tool call");

    let session = match get_session(ctx).await {
        Ok(s) => s,
        Err(msg) => return Ok(msg),
    };

    let script = match tool_call.args.get("script").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return Ok("❌ Missing script argument".to_string()),
    };

    let mut driver = session.lock().await;
    match driver.execute_script(script, vec![]).await {
        Ok(result) => Ok(format!("Script result: {:?}", result)),
        Err(e) => Ok(format!("❌ Failed to execute script: {}", e)),
    }
}

/// Execute the `webdriver_get_page_source` tool.
pub async fn execute_webdriver_get_page_source<W: UiWriter>(
    tool_call: &ToolCall,
    ctx: &ToolContext<'_, W>,
) -> Result<String> {
    debug!("Processing webdriver_get_page_source tool call");

    // Extract optional parameters
    let max_length = tool_call
        .args
        .get("max_length")
        .and_then(|v| v.as_u64())
        .map(|n| n as usize)
        .unwrap_or(10000);

    let save_to_file = tool_call.args.get("save_to_file").and_then(|v| v.as_str());

    let session = match get_session(ctx).await {
        Ok(s) => s,
        Err(msg) => return Ok(msg),
    };

    let driver = session.lock().await;
    match driver.page_source().await {
        Ok(source) => {
            // If save_to_file is specified, write to file
            if let Some(file_path) = save_to_file {
                let expanded_path = shellexpand::tilde(file_path);
                let path_str = expanded_path.as_ref();

                // Create parent directories if needed
                if let Some(parent) = std::path::Path::new(path_str).parent() {
                    if let Err(e) = std::fs::create_dir_all(parent) {
                        return Ok(format!("❌ Failed to create directories: {}", e));
                    }
                }

                match std::fs::write(path_str, &source) {
                    Ok(_) => Ok(format!(
                        "✅ Page source ({} chars) saved to: {}",
                        source.len(),
                        path_str
                    )),
                    Err(e) => Ok(format!("❌ Failed to write file: {}", e)),
                }
            } else if max_length > 0 && source.len() > max_length {
                // Truncate if max_length is set and source exceeds it
                Ok(format!(
                    "Page source ({} chars, truncated to {}):\n{}...",
                    source.len(),
                    max_length,
                    &source[..max_length]
                ))
            } else {
                // Return full source
                Ok(format!("Page source ({} chars):\n{}", source.len(), source))
            }
        }
        Err(e) => Ok(format!("❌ Failed to get page source: {}", e)),
    }
}

/// Execute the `webdriver_screenshot` tool.
pub async fn execute_webdriver_screenshot<W: UiWriter>(
    tool_call: &ToolCall,
    ctx: &ToolContext<'_, W>,
) -> Result<String> {
    debug!("Processing webdriver_screenshot tool call");

    let session = match get_session(ctx).await {
        Ok(s) => s,
        Err(msg) => return Ok(msg),
    };

    let path = match tool_call.args.get("path").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => return Ok("❌ Missing path argument".to_string()),
    };

    let mut driver = session.lock().await;
    match driver.screenshot(path).await {
        Ok(_) => Ok(format!("✅ Screenshot saved to {}", path)),
        Err(e) => Ok(format!("❌ Failed to take screenshot: {}", e)),
    }
}

/// Execute the `webdriver_back` tool.
pub async fn execute_webdriver_back<W: UiWriter>(
    tool_call: &ToolCall,
    ctx: &ToolContext<'_, W>,
) -> Result<String> {
    debug!("Processing webdriver_back tool call");
    let _ = tool_call; // unused

    let session = match get_session(ctx).await {
        Ok(s) => s,
        Err(msg) => return Ok(msg),
    };

    let mut driver = session.lock().await;
    match driver.back().await {
        Ok(_) => Ok("✅ Navigated back".to_string()),
        Err(e) => Ok(format!("❌ Failed to navigate back: {}", e)),
    }
}

/// Execute the `webdriver_forward` tool.
pub async fn execute_webdriver_forward<W: UiWriter>(
    tool_call: &ToolCall,
    ctx: &ToolContext<'_, W>,
) -> Result<String> {
    debug!("Processing webdriver_forward tool call");
    let _ = tool_call; // unused

    let session = match get_session(ctx).await {
        Ok(s) => s,
        Err(msg) => return Ok(msg),
    };

    let mut driver = session.lock().await;
    match driver.forward().await {
        Ok(_) => Ok("✅ Navigated forward".to_string()),
        Err(e) => Ok(format!("❌ Failed to navigate forward: {}", e)),
    }
}

/// Execute the `webdriver_refresh` tool.
pub async fn execute_webdriver_refresh<W: UiWriter>(
    tool_call: &ToolCall,
    ctx: &ToolContext<'_, W>,
) -> Result<String> {
    debug!("Processing webdriver_refresh tool call");
    let _ = tool_call; // unused

    let session = match get_session(ctx).await {
        Ok(s) => s,
        Err(msg) => return Ok(msg),
    };

    let mut driver = session.lock().await;
    match driver.refresh().await {
        Ok(_) => Ok("✅ Page refreshed".to_string()),
        Err(e) => Ok(format!("❌ Failed to refresh page: {}", e)),
    }
}

/// Execute the `webdriver_quit` tool.
pub async fn execute_webdriver_quit<W: UiWriter>(
    tool_call: &ToolCall,
    ctx: &ToolContext<'_, W>,
) -> Result<String> {
    debug!("Processing webdriver_quit tool call");
    let _ = tool_call; // unused

    if !ctx.config.webdriver.enabled {
        return Ok("❌ WebDriver is not enabled. Use --webdriver flag to enable.".to_string());
    }

    // Take the session
    let session = match ctx.webdriver_session.write().await.take() {
        Some(s) => s.clone(),
        None => return Ok("❌ No active WebDriver session.".to_string()),
    };

    // Quit the WebDriver session
    match std::sync::Arc::try_unwrap(session) {
        Ok(mutex) => {
            let driver = mutex.into_inner();
            match driver.quit().await {
                Ok(_) => {
                    debug!("WebDriver session closed successfully");

                    // Kill the safaridriver process
                    if let Some(mut process) = ctx.webdriver_process.write().await.take() {
                        if let Err(e) = process.kill().await {
                            warn!("Failed to kill safaridriver process: {}", e);
                        } else {
                            debug!("Safaridriver process terminated");
                        }
                    }

                    Ok("✅ WebDriver session closed and safaridriver stopped".to_string())
                }
                Err(e) => Ok(format!("❌ Failed to quit WebDriver: {}", e)),
            }
        }
        Err(_) => Ok("❌ Cannot quit: WebDriver session is still in use".to_string()),
    }
}
