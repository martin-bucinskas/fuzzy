use tokio::sync::mpsc::{Receiver, Sender};
use crate::fuzzer::fuzzy::FuzzingResult;

pub struct FuzzyResponseChannel {
  tx: Sender<FuzzingResult>,
  rx: Receiver<FuzzingResult>,
}

impl FuzzyResponseChannel {
  pub fn new(channel_size: usize) -> Self {
    let (tx, rx) = tokio::sync::mpsc::channel::<FuzzingResult>(channel_size);
    FuzzyResponseChannel {
      tx,
      rx
    }
  }

  pub fn sender(&self) -> Sender<FuzzingResult> {
    self.tx.clone()
  }

  pub fn receiver(self) -> Receiver<FuzzingResult> {
    self.rx
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use tokio::runtime;
  use crate::fuzzer::fuzzy::{FuzzedUrl, FuzzingResult};

  #[tokio::test]
  async fn test_fuzzy_response_channel_creation() {
    let channel = FuzzyResponseChannel::new(1);
    assert!(channel.sender().send(sample_fuzzing_result()).await.is_ok());
  }

  #[test]
  fn test_fuzzy_response_channel_sender() {
    let channel = FuzzyResponseChannel::new(1);
    let sender = channel.sender();

    let rt = runtime::Runtime::new().unwrap(); // Create a runtime to execute async functions
    rt.block_on(async {
      assert!(sender.send(sample_fuzzing_result()).await.is_ok());
    });
  }

  #[test]
  fn test_fuzzy_response_channel_receiver() {
    let channel = FuzzyResponseChannel::new(1);
    let sender = channel.sender();
    let mut receiver = channel.receiver();

    let rt = runtime::Runtime::new().unwrap();
    rt.block_on(async {
      sender.send(sample_fuzzing_result()).await.unwrap();
      let received = receiver.recv().await;
      assert!(received.is_some());
      assert_eq!(received.unwrap(), sample_fuzzing_result()); // Adjust this if FuzzingResult doesn't implement PartialEq
    });
  }

  fn sample_fuzzing_result() -> FuzzingResult {
    FuzzingResult::Success(FuzzedUrl::new("http://test.com/test".parse().unwrap(), "test".parse().unwrap(), "test".parse().unwrap()))
  }
}

