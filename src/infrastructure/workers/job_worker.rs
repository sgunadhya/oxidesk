use chrono::Utc;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info};

use crate::application::services::{AvailabilityService, SlaService};
use crate::domain::entities::Job;
use crate::domain::ports::oidc_repository::OidcRepository;
use crate::domain::ports::task_queue::TaskQueue;
use crate::domain::ports::webhook_repository::WebhookRepository;
use crate::shared::rate_limiter::AuthRateLimiter;

use crate::domain::ports::time_service::TimeService;

pub struct JobProcessor {
    queue: Arc<dyn TaskQueue>,
    oidc_repo: OidcRepository,
    webhook_repo: WebhookRepository,
    rate_limiter: AuthRateLimiter,
    availability_service: AvailabilityService,
    sla_service: SlaService,
    session_service: crate::application::services::SessionService,
    http_client: reqwest::Client,
    time_service: Arc<dyn TimeService>,
}

impl JobProcessor {
    pub fn new(
        queue: Arc<dyn TaskQueue>,
        oidc_repo: OidcRepository,
        webhook_repo: WebhookRepository,
        rate_limiter: AuthRateLimiter,
        availability_service: AvailabilityService,
        sla_service: SlaService,
        session_service: crate::application::services::SessionService,
        time_service: Arc<dyn TimeService>,
    ) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            queue,
            oidc_repo,
            webhook_repo,
            rate_limiter,
            availability_service,
            sla_service,
            session_service,
            http_client,
            time_service,
        }
    }

    pub async fn run(&self) {
        info!("Starting JobProcessor...");
        loop {
            match self.process_next().await {
                Ok(Some(_)) => {
                    // Job processed, check for next one immediately
                    continue;
                }
                Ok(None) => {
                    // No jobs, sleep briefly
                    self.time_service.sleep(Duration::from_secs(1)).await;
                }
                Err(e) => {
                    error!("Error processing job: {}", e);
                    self.time_service.sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }

    pub async fn process_next(&self) -> Result<Option<()>, String> {
        let job = self
            .queue
            .fetch_next_job()
            .await
            .map_err(|e| e.to_string())?;

        if let Some(job) = job {
            info!("Processing job {} (type: {})", job.id, job.job_type);

            // Execute the job logic
            let result = self.execute_job(&job).await;

            // Handle result
            match result {
                Ok(_) => {
                    info!("Job {} completed successfully", job.id);
                    if let Err(e) = self.queue.complete_job(&job.id).await {
                        error!("Failed to mark job {} as completed: {}", job.id, e);
                    }
                }
                Err(e) => {
                    error!("Job {} failed: {}", job.id, e);
                    if let Err(retry_err) = self.queue.fail_job(&job.id, &e).await {
                        error!("Failed to mark job {} as failed: {}", job.id, retry_err);
                    }
                }
            }

            Ok(Some(()))
        } else {
            Ok(None)
        }
    }

    async fn execute_job(&self, job: &Job) -> Result<(), String> {
        match job.job_type.as_str() {
            "test_job" => {
                info!("Executing test job: {:?}", job.payload);
                Ok(())
            }
            "cleanup_sessions" => self.handle_cleanup_sessions().await,
            "cleanup_rate_limiter" => self.handle_cleanup_rate_limiter().await,
            "cleanup_oidc_states" => self.handle_cleanup_oidc_states().await,
            "check_availability" => self.handle_check_availability().await,
            "check_sla_breaches" => self.handle_check_sla_breaches().await,
            "deliver_webhook" => self.handle_deliver_webhook(&job.payload).await,
            _ => Err(format!("Unknown job type: {}", job.job_type)),
        }
    }

    // --- Job Handlers ---

    async fn handle_cleanup_sessions(&self) -> Result<(), String> {
        match self.session_service.cleanup_expired_sessions().await {
            Ok(count) => {
                if count > 0 {
                    info!("Cleaned up {} expired sessions", count);
                }
                // Schedule next run in 1 hour
                let next_run = Utc::now() + chrono::Duration::hours(1);
                self.queue
                    .enqueue_at("cleanup_sessions", Value::Null, next_run, 3)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(())
            }
            Err(e) => Err(format!("Failed to cleanup sessions: {}", e)),
        }
    }

    async fn handle_cleanup_rate_limiter(&self) -> Result<(), String> {
        self.rate_limiter.cleanup().await;
        // Schedule next run in 15 minutes
        let next_run = Utc::now() + chrono::Duration::minutes(15);
        self.queue
            .enqueue_at("cleanup_rate_limiter", Value::Null, next_run, 3)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn handle_cleanup_oidc_states(&self) -> Result<(), String> {
        match self.oidc_repo.cleanup_expired_states().await {
            Ok(count) => {
                if count > 0 {
                    info!("Cleaned up {} expired OIDC states", count);
                }
                // Schedule next run in 10 minutes
                let next_run = Utc::now() + chrono::Duration::minutes(10);
                self.queue
                    .enqueue_at("cleanup_oidc_states", Value::Null, next_run, 3)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(())
            }
            Err(e) => Err(format!("Failed to cleanup OIDC states: {}", e)),
        }
    }

    async fn handle_check_availability(&self) -> Result<(), String> {
        // 1. Check inactivity
        if let Err(e) = self.availability_service.check_inactivity_timeouts().await {
            error!("Failed to check inactivity timeouts: {}", e);
        }

        // 2. Check max idle
        if let Err(e) = self.availability_service.check_max_idle_thresholds().await {
            error!("Failed to check max idle thresholds: {}", e);
        }

        // Schedule next run in 30 seconds
        let next_run = Utc::now() + chrono::Duration::seconds(30);
        self.queue
            .enqueue_at("check_availability", Value::Null, next_run, 3)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn handle_check_sla_breaches(&self) -> Result<(), String> {
        if let Err(e) = self.sla_service.check_breaches().await {
            error!("Failed to check SLA breaches: {}", e);
        }

        // Schedule next run in 60 seconds
        let next_run = Utc::now() + chrono::Duration::seconds(60);
        self.queue
            .enqueue_at("check_sla_breaches", Value::Null, next_run, 3)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn handle_deliver_webhook(&self, payload: &Value) -> Result<(), String> {
        // Extract job arguments
        let webhook_id = payload["webhook_id"]
            .as_str()
            .ok_or("Missing 'webhook_id' in job payload")?;
        let url = payload["url"]
            .as_str()
            .ok_or("Missing 'url' in job payload")?;
        let signature = payload["signature"]
            .as_str()
            .ok_or("Missing 'signature' in job payload")?;
        let event_type = payload["event_type"]
            .as_str()
            .ok_or("Missing 'event_type' in job payload")?;
        let body = payload["body"]
            .as_str()
            .ok_or("Missing 'body' in job payload")?;

        info!(
            "Attempting webhook delivery to {} for event {}",
            url, event_type
        );

        let _start = std::time::Instant::now();
        let response_result = self
            .http_client
            .post(url)
            .header("Content-Type", "application/json")
            .header("X-Webhook-Signature", signature)
            .header("X-Webhook-Event", event_type)
            .body(body.to_owned())
            .send()
            .await;

        // Log delivery to database
        // We artificially create a delivery record here to maintain logging history
        // In the old system, this record was created before attempt. Here we create it after/during.
        let mut delivery = crate::domain::entities::WebhookDelivery::new(
            webhook_id.to_string(),
            event_type.to_string(),
            body.to_owned(),
            signature.to_string(),
        );

        // Mark attempted
        delivery.attempted_at = Some(Utc::now().to_rfc3339());

        let success = match &response_result {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        };

        match response_result {
            Ok(response) => {
                let status = response.status();
                delivery.http_status_code = Some(status.as_u16() as i32);
                if status.is_success() {
                    delivery.status = crate::domain::entities::DeliveryStatus::Success;
                    delivery.completed_at = Some(Utc::now().to_rfc3339());
                    info!("Webhook delivered successfully to {}", url);
                } else {
                    delivery.status = crate::domain::entities::DeliveryStatus::Failed;
                    delivery.error_message = Some(format!("HTTP {}", status));
                }
            }
            Err(e) => {
                delivery.status = crate::domain::entities::DeliveryStatus::Failed;
                delivery.error_message = Some(e.to_string());
                info!("Webhook delivery failed: {}", e);
            }
        }

        // Save delivery record (best effort)
        if let Err(e) = self.webhook_repo.create_webhook_delivery(&delivery).await {
            error!("Failed to log webhook delivery: {}", e);
        }

        if success {
            Ok(())
        } else {
            // Return error to trigger job retry (if appropriate)
            // Note: TaskQueue retries based on Err.
            // If we want retry, we return Err.
            let error_msg = delivery
                .error_message
                .unwrap_or_else(|| "Unknown error".to_string());
            Err(error_msg)
        }
    }
}
