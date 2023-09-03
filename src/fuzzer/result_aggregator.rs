use std::fs::File;
use std::time::Instant;
use std::io::Write;
use tokio::sync::mpsc::Receiver;
use crate::fuzzer::fuzzy::FuzzingResult;
use crate::fuzzer::metrics::Metrics;

pub struct ResultAggregator {
  metrics: Metrics,
  receiver: Receiver<FuzzingResult>,
  output_file: Option<File>
}

impl ResultAggregator {
  pub async fn new(receiver: Receiver<FuzzingResult>, output_path: Option<&str>) -> Self {
    let output_file = output_path.map(|path| File::create(path).expect("failed to create output file"));

    Self {
      metrics: Metrics {
        // todo: should probably start the instant later to not skew the results...
        start_time: Instant::now(),
        total_requests: 0,
        successful_requests: 0,
        failed_requests: 0,
      },
      receiver,
      output_file,
    }
  }

  pub async fn process_results(&mut self) {
    while let Some(result) = self.receiver.recv().await {
      match result {
        FuzzingResult::Success(_url) => {
          self.metrics.successful_requests += 1;
        },
        FuzzingResult::Failure(_url, failure) => {
          self.metrics.failed_requests += 1;
          self.write_to_output(failure);
        }
      }
      self.metrics.total_requests += 1;
    }
  }

  fn write_to_output(&mut self, data: &str) {
    if let Some(file) = &mut self.output_file {
      writeln!(file, "{}", data).expect("failed to write to file");
    }
  }
}