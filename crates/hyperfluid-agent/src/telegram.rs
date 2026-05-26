// === C10 Agent Runtime: Telegram Bot Client ===
//
// Long-polling Telegram bot that responds to commands from an allowed user.
// Also supports sponsored-submission flow for human-in-the-loop approval.

use crate::config;
use reqwest::Client;
use std::time::Duration;

/// Telegram bot client wrapping the Bot API.
pub struct TelegramBot {
    // Stored for potential serialization/re-creation; used in tests.
    pub _token: String,
    pub allowed_user_id: u64,
    pub base_url: String,
    client: Client,
}

impl TelegramBot {
    /// Create a new bot from the config's `TelegramSection`.
    ///
    /// Uses the token and user_id fields. If `enabled` is false the bot
    /// will not process updates, but this is not enforced at construction time.
    pub fn new(config: &config::TelegramSection) -> Self {
        let client = match Client::builder().timeout(Duration::from_secs(30)).build() {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Telegram: failed to build reqwest client: {}", e);
                Client::new()
            }
        };
        Self {
            _token: config.token.clone(),
            allowed_user_id: config.user_id,
            base_url: format!("https://api.telegram.org/bot{}", config.token),
            client,
        }
    }

    /// Start the long-polling loop.
    ///
    /// Polls `getUpdates` with a 60-second timeout, processes messages from
    /// `allowed_user_id`, and sends replies. Opens an SQLite connection at
    /// `db_path` for the `/status` command.
    pub async fn run(&self, db_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut offset: i64 = 0;

        loop {
            let url = format!("{}/getUpdates?offset={}&timeout=60", self.base_url, offset);

            let response = self.client.get(&url).send().await;
            let body: serde_json::Value = match response {
                Ok(r) => r
                    .json()
                    .await
                    .inspect_err(|e| tracing::debug!("Telegram JSON deserialize error: {}", e))
                    .unwrap_or(serde_json::Value::Null),
                Err(e) => {
                    tracing::debug!("Telegram poll error: {}", e);
                    tokio::time::sleep(Duration::from_secs(10)).await;
                    continue;
                }
            };

            if let Some(results) = body["result"].as_array() {
                for update in results {
                    if let Some(update_id) = update["update_id"].as_i64() {
                        offset = update_id + 1;
                    }

                    if let Some(msg) = update["message"].as_object() {
                        let chat_id = msg["chat"]["id"].as_i64().unwrap_or(0) as u64;
                        let user_id = msg["from"]["id"].as_i64().unwrap_or(0) as u64;

                        // Only process messages from the allowed user
                        if user_id != self.allowed_user_id {
                            continue;
                        }

                        let text = msg["text"].as_str().unwrap_or("").to_string();
                        let reply = self.handle_command(&text, db_path).await;

                        // Send reply
                        let reply_url = format!(
                            "{}/sendMessage?chat_id={}&text={}",
                            self.base_url,
                            chat_id,
                            urlencoding(&reply)
                        );
                        if let Err(e) = self.client.get(&reply_url).send().await {
                            tracing::debug!("Telegram send error: {}", e);
                        }
                    }
                }
            }

            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    /// Handle a single command message and produce a reply text.
    async fn handle_command(&self, text: &str, db_path: &str) -> String {
        match text.trim() {
            "/start" => "Welcome to Hyperfluid! Commands: /status, /balance".to_string(),
            "/status" => self.query_status(db_path).await,
            "/balance" => {
                "Balance: check node RPC at http://127.0.0.1:8545/agent/status".to_string()
            }
            _ => "Unknown command. Available: /start, /status, /balance".to_string(),
        }
    }

    /// Query the local SQLite database for summary counts.
    async fn query_status(&self, db_path: &str) -> String {
        let conn = match rusqlite::Connection::open(db_path) {
            Ok(c) => c,
            Err(e) => return format!("Error opening database: {}", e),
        };

        let todo_count: i64 =
            conn.query_row("SELECT COUNT(*) FROM todos", [], |row| row.get(0)).unwrap_or(0);

        let knowledge_count: i64 =
            conn.query_row("SELECT COUNT(*) FROM knowledge", [], |row| row.get(0)).unwrap_or(0);

        format!(
            "📊 Agent Status\n- Todos: {}\n- Knowledge entries: {}",
            todo_count, knowledge_count
        )
    }

    /// Send a prompt to the allowed user and wait for a response containing
    /// "yes" (case-insensitive). Times out after 120 seconds.
    pub async fn run_sponsored_submission(
        &self,
        prompt: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        // 1. Send the prompt to the allowed user
        let send_url = format!(
            "{}/sendMessage?chat_id={}&text={}",
            self.base_url,
            self.allowed_user_id,
            urlencoding(prompt)
        );
        self.client.get(&send_url).send().await?;

        // 2. Poll for a response containing "yes"
        let deadline = tokio::time::Instant::now() + Duration::from_secs(120);
        let mut offset: i64 = 0;

        while tokio::time::Instant::now() < deadline {
            let url = format!("{}/getUpdates?offset={}&timeout=30", self.base_url, offset);

            let response = self.client.get(&url).send().await;
            let body: serde_json::Value = match response {
                Ok(r) => r
                    .json()
                    .await
                    .inspect_err(|e| tracing::debug!("Telegram JSON deserialize error: {}", e))
                    .unwrap_or(serde_json::Value::Null),
                Err(_) => {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };

            if let Some(results) = body["result"].as_array() {
                for update in results {
                    if let Some(update_id) = update["update_id"].as_i64() {
                        offset = update_id + 1;
                    }

                    if let Some(msg) = update["message"].as_object() {
                        let user_id = msg["from"]["id"].as_i64().unwrap_or(0) as u64;
                        if user_id != self.allowed_user_id {
                            continue;
                        }

                        let text = msg["text"].as_str().unwrap_or("");
                        if text.to_lowercase().contains("yes") {
                            return Ok(text.to_string());
                        }
                    }
                }
            }

            tokio::time::sleep(Duration::from_secs(2)).await;
        }

        Err("Timeout: no 'yes' response received within 120 seconds".into())
    }
}

/// Simple URL-encoding for Telegram message text (replaces special chars).
fn urlencoding(input: &str) -> String {
    input
        .chars()
        .map(|c| match c {
            ' ' => "%20".to_string(),
            '\n' => "%0A".to_string(),
            '&' => "%26".to_string(),
            '?' => "%3F".to_string(),
            '=' => "%3D".to_string(),
            '#' => "%23".to_string(),
            '%' => "%25".to_string(),
            '+' => "%2B".to_string(),
            '/' => "%2F".to_string(),
            _ => c.to_string(),
        })
        .collect()
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::*;

    #[test]
    fn conforms_to_agent_runtime_spec_sectionX_telegram_bot_creation() {
        let section =
            config::TelegramSection { token: "test-token-123".into(), user_id: 42, enabled: true };
        let bot = TelegramBot::new(&section);
        assert_eq!(bot._token, "test-token-123");
        assert_eq!(bot.allowed_user_id, 42);
        assert!(bot.base_url.contains("test-token-123"));
    }

    #[test]
    fn conforms_to_agent_runtime_spec_sectionX_url_encoding() {
        assert_eq!(urlencoding("hello world"), "hello%20world");
        assert_eq!(urlencoding("a&b=c"), "a%26b%3Dc");
        assert_eq!(urlencoding("plain"), "plain");
    }
}
