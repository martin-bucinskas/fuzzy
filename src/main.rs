use std::fs::{read_dir, read_to_string};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use log::LevelFilter;
use crate::domain::dictionary::FuzzyDictionary;
use crate::domain::input::{FuzzyInput};
use crate::fuzzer::data_channels::FuzzyResponseChannel;
use crate::fuzzer::fuzzy::Fuzzer;
use crate::fuzzer::metrics::Metrics;
use crate::fuzzer::result_aggregator::ResultAggregator;

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

    // let strings = std::fs::read_to_string("./dictionary/strings.yml")
    //   .expect("failed to read dictionary");
    // let strings_dict: FuzzyDictionary = serde_yaml::from_str(&strings)
    //   .expect("failed to parse dictionary yaml");

    let dictionary = load_dictionaries_from_dir("./dictionary");

    log::info!("loaded...");

    let finished = Arc::new(AtomicBool::new(false));
    let fuzzer_finished = finished.clone();

    let response_channel = FuzzyResponseChannel::new(32);

    let shared_metrics = Metrics::new();
    let shared_metrics_clone = shared_metrics.clone();

    let fuzzer = Fuzzer::new( 10, response_channel.sender());
    let mut aggregator = ResultAggregator::new(response_channel.receiver(), Some("output.txt"), shared_metrics.clone()).await;

    let aggregator_task = tokio::spawn(async move {
        aggregator.process_results().await
    });

    let display_task = tokio::spawn(async move {
        while !finished.load(Ordering::Relaxed) {
            // reading metrics into a local variable to release the lock before sleeping
            // this prevents lock from blocking the fuzzer thread
            let local_metrics = {
              let metrics = shared_metrics.read().await;
                metrics.clone()
            };

            local_metrics.display();
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    });

    let fuzzer_task = tokio::spawn(async move {
        log::info!("fuzzing...");
        fuzzer.fuzz(&input_data, &dictionary).await;
        fuzzer_finished.store(true, Ordering::Relaxed);
        log::info!("fuzzing finished");
    });

    let task_join = tokio::try_join!(aggregator_task, display_task, fuzzer_task);

    match task_join {
        Ok((aggregator_res, display_res, fuzzer_res)) => {
            log::info!("all tasks joined...");
            log::debug!("aggregator task returned: {:?}", aggregator_res);
            log::debug!("display task returned: {:?}", display_res);
            log::debug!("fuzzer task returned: {:?}", fuzzer_res);
        },
        Err(join_error) => {
            log::error!("task panicked: {}", join_error);
        }
    }

    let final_metrics = shared_metrics_clone.read().await;
    final_metrics.display();

    Ok(())
}

fn load_dictionaries_from_dir<P: AsRef<Path>>(dir_path: P) -> FuzzyDictionary {
    let mut dictionaries = Vec::new();

    if let Ok(entries) = read_dir(dir_path) {
        for entry in entries.flatten() {
            if entry.path().is_file() {
                let content = read_to_string(&entry.path())
                  .unwrap_or_else(|_| panic!("failed to read dictionary from {:?}", entry.path()));

                let dict: FuzzyDictionary = serde_yaml::from_str(&content)
                  .unwrap_or_else(|_| panic!("failed to parse dictionary yaml from {:?}", entry.path()));

                dictionaries.extend(dict.data().clone());
            }
        }
    } else {
        log::error!("failed to read dictionary directory")
    }

    FuzzyDictionary::new(dictionaries)
}
