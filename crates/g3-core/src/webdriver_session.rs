//! Unified WebDriver session abstraction.
//!
//! This module provides a unified interface for browser automation
//! that can work with either Safari or Chrome WebDriver.

use g3_computer_control::{ChromeDriver, SafariDriver, WebDriverController, WebElement};

/// Unified WebDriver session that can hold either Safari or Chrome driver.
pub enum WebDriverSession {
    Safari(SafariDriver),
    Chrome(ChromeDriver),
}

#[async_trait::async_trait]
impl WebDriverController for WebDriverSession {
    async fn navigate(&mut self, url: &str) -> anyhow::Result<()> {
        match self {
            WebDriverSession::Safari(driver) => driver.navigate(url).await,
            WebDriverSession::Chrome(driver) => driver.navigate(url).await,
        }
    }

    async fn current_url(&self) -> anyhow::Result<String> {
        match self {
            WebDriverSession::Safari(driver) => driver.current_url().await,
            WebDriverSession::Chrome(driver) => driver.current_url().await,
        }
    }

    async fn title(&self) -> anyhow::Result<String> {
        match self {
            WebDriverSession::Safari(driver) => driver.title().await,
            WebDriverSession::Chrome(driver) => driver.title().await,
        }
    }

    async fn find_element(
        &mut self,
        selector: &str,
    ) -> anyhow::Result<WebElement> {
        match self {
            WebDriverSession::Safari(driver) => driver.find_element(selector).await,
            WebDriverSession::Chrome(driver) => driver.find_element(selector).await,
        }
    }

    async fn find_elements(
        &mut self,
        selector: &str,
    ) -> anyhow::Result<Vec<WebElement>> {
        match self {
            WebDriverSession::Safari(driver) => driver.find_elements(selector).await,
            WebDriverSession::Chrome(driver) => driver.find_elements(selector).await,
        }
    }

    async fn execute_script(
        &mut self,
        script: &str,
        args: Vec<serde_json::Value>,
    ) -> anyhow::Result<serde_json::Value> {
        match self {
            WebDriverSession::Safari(driver) => driver.execute_script(script, args).await,
            WebDriverSession::Chrome(driver) => driver.execute_script(script, args).await,
        }
    }

    async fn page_source(&self) -> anyhow::Result<String> {
        match self {
            WebDriverSession::Safari(driver) => driver.page_source().await,
            WebDriverSession::Chrome(driver) => driver.page_source().await,
        }
    }

    async fn screenshot(&mut self, path: &str) -> anyhow::Result<()> {
        match self {
            WebDriverSession::Safari(driver) => driver.screenshot(path).await,
            WebDriverSession::Chrome(driver) => driver.screenshot(path).await,
        }
    }

    async fn close(&mut self) -> anyhow::Result<()> {
        match self {
            WebDriverSession::Safari(driver) => driver.close().await,
            WebDriverSession::Chrome(driver) => driver.close().await,
        }
    }

    async fn quit(self) -> anyhow::Result<()> {
        match self {
            WebDriverSession::Safari(driver) => driver.quit().await,
            WebDriverSession::Chrome(driver) => driver.quit().await,
        }
    }
}

// Additional methods for WebDriverSession that aren't part of the WebDriverController trait
impl WebDriverSession {
    pub async fn back(&mut self) -> anyhow::Result<()> {
        match self {
            WebDriverSession::Safari(driver) => driver.back().await,
            WebDriverSession::Chrome(driver) => driver.back().await,
        }
    }

    pub async fn forward(&mut self) -> anyhow::Result<()> {
        match self {
            WebDriverSession::Safari(driver) => driver.forward().await,
            WebDriverSession::Chrome(driver) => driver.forward().await,
        }
    }

    pub async fn refresh(&mut self) -> anyhow::Result<()> {
        match self {
            WebDriverSession::Safari(driver) => driver.refresh().await,
            WebDriverSession::Chrome(driver) => driver.refresh().await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webdriver_session_enum_variants() {
        // This test just verifies the enum structure compiles correctly
        // Actual WebDriver tests would require a running browser
        fn _assert_send<T: Send>() {}
        fn _assert_sync<T: Sync>() {}
        // WebDriverSession should be Send but not necessarily Sync due to internal state
    }
}
