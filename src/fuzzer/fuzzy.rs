use std::str::FromStr;
use std::sync::Arc;
use reqwest::{Client, Response, StatusCode};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use tokio::sync::{RwLock, Semaphore};
use tokio::sync::mpsc::Sender;
use url::{ParseError, Url};
use crate::domain::dictionary::FuzzyDictionary;
use crate::domain::input::{FuzzyInput, HttpMethod, Path};
use crate::fuzzer::metrics::Metrics;

const FUZZING_PLACEHOLDER: &str = "%7Bfuzz%7D";

pub struct FuzzingFailure {
  network_error: Option<reqwest::Error>,
  status_code: Option<u16>,
  response: Option<Response>,
}

impl FuzzingFailure {
  fn new(network_error: Option<reqwest::Error>, status_code: Option<u16>, response: Option<Response>) -> Self {
    Self {
      network_error,
      status_code,
      response,
    }
  }
}

pub enum FuzzingResult {
  Success(FuzzedUrl),
  Failure(FuzzedUrl, FuzzingFailure),
}

#[derive(Clone)]
pub struct FuzzedUrl {
  url: String,
  description: String,
  id: String,
}

impl FuzzedUrl {
  pub fn new(url: String, description: String, id: String) -> Self {
    Self {
      url,
      description,
      id,
    }
  }

  fn url(&self) -> &String {
    &self.url
  }

  fn description(&self) -> &String {
    &self.description
  }

  fn id(&self) -> &String {
    &self.id
  }
}

#[derive(Clone)]
pub struct Fuzzer {
  client: Client,
  metrics: Arc<RwLock<Metrics>>,
  semaphore: Arc<Semaphore>,
  tx: Sender<FuzzingResult>,
}

impl Fuzzer {
  pub fn new(metrics: Arc<RwLock<Metrics>>, num_of_concurrent_requests: usize, tx: Sender<FuzzingResult>) -> Self {
    Fuzzer {
      client: Client::new(),
      metrics,
      semaphore: Arc::new(Semaphore::new(num_of_concurrent_requests)),
      tx,
    }
  }

  fn generate_url(&self, input_data: &FuzzyInput, path: &Path) -> Result<Url, ParseError> {
    path.to_url(input_data.host(), input_data.base_path())
      .map_err(|err| {
        log::error!("failed to parse url: {}", err);
        err
      })
  }

  fn generate_fuzzed_urls(&self, url: &Url, dict: &FuzzyDictionary) -> Vec<FuzzedUrl> {
    let mut fuzzed_urls = Vec::new();
    for item in dict.data() {
      for fuzz_param in item.values() {
        let fuzzed_url = url.to_string().replace(FUZZING_PLACEHOLDER, fuzz_param);
        fuzzed_urls.push(FuzzedUrl {
          url: fuzzed_url,
          description: item.description().to_string(),
          id: item.id().to_string(),
        });
      }
    }

    fuzzed_urls
  }

  async fn make_request(&self, fuzzed_url: &FuzzedUrl, path: &Path) -> Result<Response, reqwest::Error> {
    let _permit = self.semaphore.acquire().await;
    let mut headers = HeaderMap::new();

    for header in path.headers() {
      headers.insert(
        HeaderName::from_str(header.name().as_str()).expect("invalid header name provided"),
        HeaderValue::from_str(header.value().as_ref().expect("missing header value").as_str()).expect("invalid header value provided"),
      );
    }

    log::trace!("making request: {}", fuzzed_url.url);

    self.client.get(fuzzed_url.url())
      .headers(headers)
      .send()
      .await
  }

  async fn log_metrics(&self, response: Result<Response, reqwest::Error>, fuzzed_url: &FuzzedUrl, path: &Path) {
    match response {
      Ok(success) => {
        let mut metrics = self.metrics.write().await;
        if success.status().as_u16() != path.expected_status().clone() {
          let fuzzing_failure = FuzzingFailure::new(Option::None, Option::Some(success.status().as_u16()), Option::Some(success));
          self.tx.send(FuzzingResult::Failure(fuzzed_url.clone(), fuzzing_failure)).await.unwrap();
          // log::info!("\t➡️ received a non expected status code: {}\ndescription: ({}) {}\nurl: {}", success.status().as_u16(), fuzzed_url.id, fuzzed_url.description, fuzzed_url.url);
          metrics.failed_requests += 1;
        } else {
          self.tx.send(FuzzingResult::Success(fuzzed_url.clone())).await.unwrap();
          metrics.successful_requests += 1;
        }
        metrics.total_requests += 1;
      }
      Err(err) => {
        let fuzzing_failure = FuzzingFailure::new(Option::Some(err), Option::None, Option::None);
        self.tx.send(FuzzingResult::Failure(fuzzed_url.clone(),fuzzing_failure)).await.unwrap();
        // log::info!("\t➡️ received an error whilst fuzzing: {}\ndescription: ({}) {}\nurl: {}", err, fuzzed_url.id, fuzzed_url.description, fuzzed_url.url);
        let mut metrics = self.metrics.write().await;
        metrics.failed_requests += 1;
        metrics.total_requests += 1;
      }
    }
  }

  pub async fn fuzz(&self, input_data: &FuzzyInput, dict: &FuzzyDictionary) {
    // vec to hold JoinHandle of each spawned task
    let mut path_handles = Vec::new();

    for path in input_data.paths().clone() {
      let input_data_clone = input_data.clone();
      let dict_clone = dict.clone();
      let self_clone = self.clone();

      let handle = tokio::spawn(async move {
        if let Ok(url) = self_clone.generate_url(&input_data_clone, &path) {
          let fuzzed_urls = self_clone.generate_fuzzed_urls(&url, &dict_clone);
          let mut request_handles = Vec::new();

          for fuzzed_url in fuzzed_urls {
            if path.method() == &HttpMethod::GET {
              let fuzzed_url_clone = fuzzed_url.clone();
              let path_clone = path.clone();
              let self_clone_inner = self_clone.clone();

              let request_handle = tokio::spawn(async move {
                let response = self_clone_inner.make_request(&fuzzed_url_clone, &path_clone).await;
                self_clone_inner.log_metrics( response, &fuzzed_url_clone, &path_clone).await;
              });

              request_handles.push(request_handle);
            }
          }

          for handle in request_handles {
            handle.await.unwrap();
          }
        }
      });

      path_handles.push(handle);
    }

    for handle in path_handles {
      handle.await.unwrap();
    }

    for path in input_data.paths() {
      if let Ok(url) = self.generate_url(input_data, path) {
        let fuzzed_urls = self.generate_fuzzed_urls(&url, dict);
        for fuzzed_url in fuzzed_urls {
          if path.method() == &HttpMethod::GET {
            let response = self.make_request(&fuzzed_url, path).await;
            self.log_metrics(response, &fuzzed_url, path).await;
          }
        }
      }
    }
  }
}
