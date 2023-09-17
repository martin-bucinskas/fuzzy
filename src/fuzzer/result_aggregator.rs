use std::fs::File;
use std::io::Write;
use std::sync::Arc;
use tokio::sync::mpsc::Receiver;
use tokio::sync::RwLock;
use crate::fuzzer::fuzzy::FuzzingResult;
use crate::fuzzer::metrics::Metrics;

pub struct ResultAggregator {
  metrics: Arc<RwLock<Metrics>>,
  receiver: Receiver<FuzzingResult>,
  output_file: Option<File>
}

impl ResultAggregator {
  pub async fn new(receiver: Receiver<FuzzingResult>, output_path: Option<&str>, metrics: Arc<RwLock<Metrics>>) -> Self {
    let output_file = output_path.map(|path| File::create(path).expect("failed to create output file"));

    Self {
      metrics,
      receiver,
      output_file,
    }
  }

  pub fn metrics(&self) -> &Arc<RwLock<Metrics>> {
    &self.metrics
  }

  pub async fn process_results(&mut self) {
    while let Some(result) = self.receiver.recv().await {
      match result {
        FuzzingResult::Success(_url) => {
          let mut metrics = self.metrics.write().await;
          metrics.successful_requests += 1;
        },
        FuzzingResult::Failure(url, failure) => {
          {
            let mut metrics = self.metrics.write().await;
            metrics.failed_requests += 1;
          }
          self.write_to_output(failure.failure_to_string(url).as_str());
        }
      }
      let mut metrics = self.metrics.write().await;
      metrics.total_requests += 1;
    }
  }

  fn write_to_output(&mut self, data: &str) {
    if let Some(file) = &mut self.output_file {
      writeln!(file, "{}", data).expect("failed to write to file");
    }
  }
}

mod tests {
  use super::*;

  use tokio::sync::mpsc;
  use crate::fuzzer::fuzzy::{FuzzingResult, FuzzingFailure, FuzzedUrl};

  #[tokio::test]
  async fn test_result_aggregator_initialization() {
    let (_tx, rx) = mpsc::channel(32);
    let metrics = Metrics::new();
    let aggregator = ResultAggregator::new(rx, None, metrics.clone()).await;

    assert_eq!(aggregator.metrics().read().await.successful_requests, 0);
  }

  #[tokio::test]
  async fn test_result_aggregator_process_results() {
    let (tx, rx) = mpsc::channel(32);
    let metrics = Metrics::new();
    let mut aggregator = ResultAggregator::new(rx, None, metrics.clone()).await;

    tx.send(sample_fuzzing_success_result()).await.unwrap();
    tx.send(sample_fuzzing_failure_result()).await.unwrap();

    drop(tx);
    aggregator.process_results().await;  // Ideally, you'd want this to run in parallel or ensure all messages are processed.

    let m = aggregator.metrics().read().await;
    assert_eq!(m.successful_requests, 1);
    assert_eq!(m.failed_requests, 1);
    assert_eq!(m.total_requests, 2);
  }

  fn sample_fuzzing_success_result() -> FuzzingResult {
    FuzzingResult::Success(FuzzedUrl::new("http://test.com/test".parse().unwrap(), "test".parse().unwrap(), "test".parse().unwrap()))
  }

  fn sample_fuzzing_failure_result() -> FuzzingResult {
    FuzzingResult::Failure(
      FuzzedUrl::new("http://test.com/test".parse().unwrap(), "test".parse().unwrap(), "test".parse().unwrap()),
      FuzzingFailure::new(Option::None, Option::None, Option::None)
    )
  }
}
