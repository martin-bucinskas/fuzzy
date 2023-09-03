use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use log::LevelFilter;
use crate::domain::dictionary::FuzzyDictionary;
use crate::domain::input::{FuzzyInput};
use crate::fuzzer::data_channels::FuzzyResponseChannel;
use crate::fuzzer::fuzzy::Fuzzer;
use crate::fuzzer::metrics::Metrics;

mod cli;
mod domain;
mod fuzzer;

#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    env_logger::Builder::new()
      .filter(None, LevelFilter::Info)
      .init();

    log::info!("fuzzy v{}", env!("CARGO_PKG_VERSION"));
    let args = cli::cli::CliFuzzEndpoint::parse();

    let input_file = &args.input_file;

    let input_file_str = std::fs::read_to_string(input_file)
      .expect("could not find the input file");
    let input_data: FuzzyInput = serde_yaml::from_str(&input_file_str)
      .expect("failed to parse input yaml");

    let strings = std::fs::read_to_string("./dictionary/strings.yml")
      .expect("failed to read dictionary");
    let strings_dict: FuzzyDictionary = serde_yaml::from_str(&strings)
      .expect("failed to parse dictionary yaml");

    log::info!("loaded...");

    let finished = Arc::new(AtomicBool::new(false));
    let fuzzer_finished = finished.clone();

    let response_channel = FuzzyResponseChannel::new(32);

    let metrics = Metrics::new();
    let display_metrics = metrics.clone();

    let fuzzer = Fuzzer::new(metrics.clone(), 10, response_channel.sender());

    let display_task = tokio::spawn(async move {
        while !finished.load(Ordering::Relaxed) {
            // reading metrics into a local variable to release the lock before sleeping
            // this prevents lock from blocking the fuzzer thread
            let local_metrics = {
              let metrics = display_metrics.read().await;
                metrics.clone()
            };

            local_metrics.display();
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    });

    let fuzzer_task = tokio::spawn(async move {
        log::info!("fuzzing...");
        fuzzer.fuzz(&input_data, &strings_dict).await;
        fuzzer_finished.store(true, Ordering::Relaxed);
        log::info!("fuzzing finished");
    });

    fuzzer_task.await.unwrap();
    display_task.await.unwrap();

    let final_metrics = metrics.read().await;
    final_metrics.display();

    Ok(())
}
