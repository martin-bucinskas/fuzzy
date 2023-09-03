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

  pub fn receiver(&self) -> &Receiver<FuzzingResult> {
    &self.rx
  }
}
