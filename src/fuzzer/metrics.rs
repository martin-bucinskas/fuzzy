use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{RwLock};


#[derive(Clone, Copy)]
pub struct Metrics {
  pub start_time: Instant,
  pub successful_requests: u64,
  pub failed_requests: u64,
  pub total_requests: u64,
}

impl Metrics {
  pub fn new() -> Arc<RwLock<Self>> {
    Arc::new(RwLock::new(Self {
      start_time: Instant::now(),
      successful_requests: 0,
      failed_requests: 0,
      total_requests: 0,
    }))
  }

  pub fn display(&self) {
    let elapsed_seconds = self.start_time.clone().elapsed().as_secs_f64();

    let throughput = if elapsed_seconds == 0.0 {
      0.0
    } else {
      self.total_requests.clone() as f64 / elapsed_seconds.clone()
    };

    log::info!("============================================");
    log::info!("ℹ️ total requests: {}", self.total_requests);
    log::info!("ℹ️ successful requests: {}", self.successful_requests);
    log::info!("ℹ️ failed requests: {}", self.failed_requests);
    log::info!("ℹ️ throughput: {:.2} req/s", throughput);
    log::info!("ℹ️ time: {:.2} s", elapsed_seconds);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn test_metrics_initialization() {
    let metrics_lock = Metrics::new();
    let metrics = metrics_lock.read().await;

    assert_eq!(metrics.successful_requests, 0);
    assert_eq!(metrics.failed_requests, 0);
    assert_eq!(metrics.total_requests, 0);
  }

  #[tokio::test]
  async fn test_display() {
    let metrics_arc = Metrics::new();
    {
      let mut metrics = metrics_arc.write().await;
      metrics.successful_requests = 10;
      metrics.failed_requests = 5;
      metrics.total_requests = 15;
    } // Drop the lock

    metrics_arc.read().await.display();
  }
}
