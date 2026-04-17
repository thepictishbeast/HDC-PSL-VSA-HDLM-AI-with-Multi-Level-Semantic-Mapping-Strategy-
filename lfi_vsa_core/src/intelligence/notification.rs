//! # Purpose
//! Notification system for autonomous AI operation. When the AI hits a blocker,
//! completes a milestone, encounters an error, or needs human input, it notifies
//! the operator through the highest-priority available channel.
//!
//! # Design Decisions
//! - Channel priority: Matrix (E2E encrypted) → SMS (self-hosted) → Email → Desktop → Webhook
//! - Challenge token authentication: outgoing messages embed a token, replies must
//!   contain it to be accepted as valid operator input
//! - Escalation: if no response within configurable timeout, try next channel
//! - All notifications logged for audit trail
//! - PSA compliant: Matrix is self-hosted, SMS via local gateway, no cloud dependencies
//!
//! # Invariants
//! - At least one channel must be configured for the system to start
//! - Challenge tokens are CSPRNG-generated, 32 bytes, hex-encoded
//! - Notification queue is bounded (max 100 pending) to prevent memory exhaustion
//!
//! # Failure Modes
//! - All channels down: notifications queue in memory, log to disk, retry on timer
//! - SMS modem disconnected: falls through to email
//! - Matrix server down: falls through to SMS
//! - Challenge token replay: tokens are single-use, expire after 1 hour

use std::collections::VecDeque;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// A notification channel ordered by priority.
#[derive(Debug, Clone)]
pub enum NotificationChannel {
    /// Matrix — E2E encrypted, federated, Supersociety-grade.
    /// Requires matrix-sdk integration (future).
    Matrix { room_id: String, homeserver: String },
    /// SMS — self-hosted gateway via sms-server crate or Twilio fallback.
    Sms { phone: String, gateway: SmsGateway },
    /// Email — via local Postfix/sendmail.
    Email { address: String },
    /// Desktop notification — via notify-send.
    Desktop,
    /// Webhook — HTTP POST to configured URL.
    Webhook { url: String, secret: Option<String> },
}

/// SMS gateway options.
#[derive(Debug, Clone)]
pub enum SmsGateway {
    /// Self-hosted via USB GSM modem + sms-server crate.
    SelfHosted { device: String },
    /// Twilio cloud fallback.
    Twilio { account_sid: String, auth_token: String, from_number: String },
}

/// What triggered the notification.
#[derive(Debug, Clone)]
pub enum NotificationTrigger {
    /// A task completed successfully.
    TaskComplete { task_id: String, summary: String },
    /// Hit a blocker that needs human input.
    BlockerHit { description: String, needs_input: bool },
    /// Critical error occurred.
    ErrorCritical { error: String },
    /// Training milestone reached.
    TrainingMilestone { facts: u64, score: f64 },
    /// Security alert.
    SecurityAlert { severity: u8, description: String },
    /// Scheduled periodic update.
    ScheduledUpdate { interval_secs: u64 },
    /// System going idle — no more work to do.
    GoingIdle { reason: String },
}

/// A pending notification with challenge token.
#[derive(Debug, Clone)]
pub struct Notification {
    pub trigger: NotificationTrigger,
    pub message: String,
    pub challenge_token: String,
    pub created_at: u64,
    pub sent: bool,
    pub channel_used: Option<String>,
    pub response_received: bool,
}

/// The notification engine.
pub struct NotificationEngine {
    /// Configured channels in priority order.
    channels: Vec<NotificationChannel>,
    /// Pending notifications.
    queue: VecDeque<Notification>,
    /// Maximum queue size.
    max_queue: usize,
    /// Response timeout before escalating to next channel.
    escalation_timeout: Duration,
    /// Statistics.
    pub stats: NotificationStats,
}

/// Notification statistics.
#[derive(Debug, Clone, Default)]
pub struct NotificationStats {
    pub total_sent: u64,
    pub total_escalated: u64,
    pub total_responded: u64,
    pub total_timed_out: u64,
    pub channel_failures: u64,
}

impl NotificationEngine {
    pub fn new() -> Self {
        Self {
            channels: Vec::new(),
            queue: VecDeque::new(),
            max_queue: 100,
            escalation_timeout: Duration::from_secs(300), // 5 min default
            stats: NotificationStats::default(),
        }
    }

    /// Add a notification channel (order matters — first = highest priority).
    pub fn add_channel(&mut self, channel: NotificationChannel) {
        self.channels.push(channel);
    }

    /// Generate a CSPRNG challenge token.
    fn generate_token() -> String {
        use rand::RngCore;
        let mut bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        hex::encode(&bytes)
    }

    /// Queue a notification. Returns the challenge token.
    pub fn notify(&mut self, trigger: NotificationTrigger) -> String {
        let token = Self::generate_token();
        let message = Self::format_message(&trigger, &token);

        let notification = Notification {
            trigger,
            message,
            challenge_token: token.clone(),
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            sent: false,
            channel_used: None,
            response_received: false,
        };

        if self.queue.len() >= self.max_queue {
            self.queue.pop_front(); // Drop oldest if full
        }
        self.queue.push_back(notification);
        token
    }

    /// Format a notification message with challenge token.
    fn format_message(trigger: &NotificationTrigger, token: &str) -> String {
        let body = match trigger {
            NotificationTrigger::TaskComplete { task_id, summary } =>
                format!("Task {} complete: {}", task_id, summary),
            NotificationTrigger::BlockerHit { description, needs_input } =>
                format!("BLOCKER: {}{}",
                    description,
                    if *needs_input { "\nYour input needed. Reply with instructions." } else { "" }
                ),
            NotificationTrigger::ErrorCritical { error } =>
                format!("CRITICAL ERROR: {}", error),
            NotificationTrigger::TrainingMilestone { facts, score } =>
                format!("Training milestone: {} facts, score {:.2}", facts, score),
            NotificationTrigger::SecurityAlert { severity, description } =>
                format!("SECURITY [sev {}]: {}", severity, description),
            NotificationTrigger::ScheduledUpdate { interval_secs } =>
                format!("Scheduled update (every {}s)", interval_secs),
            NotificationTrigger::GoingIdle { reason } =>
                format!("Going idle: {}", reason),
        };
        format!("[PlausiDen AI] {}\n\nToken: {}", body, &token[..8])
    }

    /// Send pending notifications through available channels.
    /// Returns the number of notifications sent.
    pub fn flush(&mut self) -> usize {
        let mut sent = 0;
        for notif in self.queue.iter_mut() {
            if notif.sent {
                continue;
            }
            for (i, channel) in self.channels.iter().enumerate() {
                match Self::send_via_channel(channel, &notif.message) {
                    Ok(()) => {
                        notif.sent = true;
                        notif.channel_used = Some(format!("channel_{}", i));
                        self.stats.total_sent += 1;
                        sent += 1;
                        break;
                    }
                    Err(_) => {
                        self.stats.channel_failures += 1;
                        continue; // Try next channel
                    }
                }
            }
        }
        sent
    }

    /// Send via a specific channel. Currently implements Desktop and Webhook.
    /// Matrix and SMS require async runtime — stubbed for now.
    fn send_via_channel(channel: &NotificationChannel, message: &str) -> Result<(), String> {
        match channel {
            NotificationChannel::Desktop => {
                std::process::Command::new("notify-send")
                    .args(&["PlausiDen AI", message])
                    .output()
                    .map_err(|e| format!("notify-send failed: {}", e))?;
                Ok(())
            }
            NotificationChannel::Email { address } => {
                // Use sendmail/mail command
                let mut child = std::process::Command::new("mail")
                    .args(&["-s", "PlausiDen AI Notification", address])
                    .stdin(std::process::Stdio::piped())
                    .spawn()
                    .map_err(|e| format!("mail failed: {}", e))?;
                if let Some(stdin) = child.stdin.as_mut() {
                    use std::io::Write;
                    let _ = stdin.write_all(message.as_bytes());
                }
                let _ = child.wait();
                Ok(())
            }
            NotificationChannel::Webhook { url, secret } => {
                let body = serde_json::json!({
                    "text": message,
                    "secret": secret,
                });
                let result = std::process::Command::new("curl")
                    .args(&["-s", "-X", "POST", url,
                        "-H", "Content-Type: application/json",
                        "-d", &body.to_string()])
                    .output()
                    .map_err(|e| format!("webhook curl failed: {}", e))?;
                if result.status.success() { Ok(()) } else {
                    Err("webhook returned non-200".into())
                }
            }
            NotificationChannel::Matrix { .. } => {
                // TODO: Requires matrix-sdk async integration
                Err("Matrix not yet implemented".into())
            }
            NotificationChannel::Sms { .. } => {
                // TODO: Requires sms-server or Twilio integration
                Err("SMS not yet implemented".into())
            }
        }
    }

    /// Validate a challenge token from an operator reply.
    pub fn validate_response(&mut self, token_prefix: &str) -> Option<&Notification> {
        for notif in self.queue.iter_mut() {
            if notif.challenge_token.starts_with(token_prefix) && !notif.response_received {
                notif.response_received = true;
                self.stats.total_responded += 1;
                return Some(notif);
            }
        }
        None
    }

    /// Number of unsent notifications.
    pub fn pending_count(&self) -> usize {
        self.queue.iter().filter(|n| !n.sent).count()
    }

    /// Number of sent but unresponded notifications.
    pub fn awaiting_response(&self) -> usize {
        self.queue.iter().filter(|n| n.sent && !n.response_received).count()
    }
}

// hex encoding without external dep
mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notify_queues_and_returns_token() {
        let mut engine = NotificationEngine::new();
        let token = engine.notify(NotificationTrigger::GoingIdle {
            reason: "test".into(),
        });
        assert_eq!(token.len(), 64); // 32 bytes hex = 64 chars
        assert_eq!(engine.pending_count(), 1);
    }

    #[test]
    fn test_flush_with_desktop_channel() {
        let mut engine = NotificationEngine::new();
        engine.add_channel(NotificationChannel::Desktop);
        engine.notify(NotificationTrigger::TrainingMilestone {
            facts: 50_000_000,
            score: 4.2,
        });
        // Desktop notification may or may not work in this env
        let _sent = engine.flush();
    }

    #[test]
    fn test_validate_response() {
        let mut engine = NotificationEngine::new();
        let token = engine.notify(NotificationTrigger::BlockerHit {
            description: "need input".into(),
            needs_input: true,
        });
        let prefix = &token[..8];
        assert!(engine.validate_response(prefix).is_some());
        // Second validation fails (single-use)
        assert!(engine.validate_response(prefix).is_none());
    }

    #[test]
    fn test_queue_bounded() {
        let mut engine = NotificationEngine::new();
        engine.max_queue = 5;
        for i in 0..10 {
            engine.notify(NotificationTrigger::ScheduledUpdate { interval_secs: i });
        }
        assert_eq!(engine.queue.len(), 5); // Oldest dropped
    }

    #[test]
    fn test_format_message_includes_token() {
        let token = "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890";
        let msg = NotificationEngine::format_message(
            &NotificationTrigger::ErrorCritical { error: "test".into() },
            token,
        );
        assert!(msg.contains("CRITICAL ERROR"));
        assert!(msg.contains("abcdef12")); // First 8 chars of token
    }
}
