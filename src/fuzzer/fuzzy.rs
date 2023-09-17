use std::str::FromStr;
use std::sync::Arc;
use async_trait::async_trait;
use reqwest::{Client, Error, Response};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use tokio::sync::Semaphore;
use tokio::sync::mpsc::Sender;
use url::{ParseError, Url};
use uuid::uuid;
use crate::domain::dictionary::FuzzyDictionary;
use crate::domain::input::{FuzzyInput, HttpMethod, Path};

const FUZZING_PLACEHOLDER: &str = "%7Bfuzz%7D";

#[async_trait]
trait HttpClient {
  async fn get(&self, url: &str) -> Result<Response, Error>;
}

#[async_trait]
impl HttpClient for Client {
  async fn get(&self, url: &str) -> Result<Response, Error> {
    self.get(url).send().await
  }
}

#[derive(Debug)]
pub struct FuzzingFailure {
  network_error: Option<reqwest::Error>,
  status_code: Option<u16>,
  response: Option<Response>,
}

impl PartialEq for FuzzingFailure {
  fn eq(&self, other: &Self) -> bool {
    let neq = self.network_error.is_some() == other.network_error.is_some();
    let seq = self.status_code == other.status_code;
    let req = self.response.is_some() == other.response.is_some();

    neq && seq && req
  }
}

impl FuzzingFailure {
  pub fn new(network_error: Option<reqwest::Error>, status_code: Option<u16>, response: Option<Response>) -> Self {
    Self {
      network_error,
      status_code,
      response,
    }
  }

  pub fn failure_to_string(&self, url: FuzzedUrl) -> String {
    format!("id: {}, url: {}, status_code: {:?}, response: {:?}, network_error: {:?}", url.id(), url.url(), self.status_code, self.response, self.network_error)
  }
}

#[derive(PartialEq, Debug)]
pub enum FuzzingResult {
  Success(FuzzedUrl),
  Failure(FuzzedUrl, FuzzingFailure),
}

#[derive(Clone, Debug, PartialEq)]
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

  pub fn url(&self) -> &String {
    &self.url
  }

  pub fn description(&self) -> &String {
    &self.description
  }

  pub fn id(&self) -> &String {
    &self.id
  }
}

#[derive(Clone)]
pub struct Fuzzer {
  client: Client,
  semaphore: Arc<Semaphore>,
  tx: Sender<FuzzingResult>,
}

impl Fuzzer {
  pub fn new(num_of_concurrent_requests: usize, tx: Sender<FuzzingResult>) -> Self {
    Fuzzer {
      client: Client::new(),
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
    let id = uuid::Uuid::new_v4();
    match response {
      Ok(success) => {
        // let mut metrics = self.metrics.write().await;
        if success.status().as_u16() != path.expected_status().clone() {
          log::info!("Failure!!!! {}", id);
          let fuzzing_failure = FuzzingFailure::new(Option::None, Option::Some(success.status().as_u16()), Option::Some(success));
          self.tx.send(FuzzingResult::Failure(fuzzed_url.clone(), fuzzing_failure)).await.unwrap();
          // log::info!("\t➡️ received a non expected status code: {}\ndescription: ({}) {}\nurl: {}", success.status().as_u16(), fuzzed_url.id, fuzzed_url.description, fuzzed_url.url);
          // metrics.failed_requests += 1;
        } else {
          self.tx.send(FuzzingResult::Success(fuzzed_url.clone())).await.unwrap();
          // metrics.successful_requests += 1;
        }
        // metrics.total_requests += 1;
      }
      Err(err) => {
        log::info!("Another Failure!!!! {}", id);
        let fuzzing_failure = FuzzingFailure::new(Option::Some(err), Option::None, Option::None);
        self.tx.send(FuzzingResult::Failure(fuzzed_url.clone(),fuzzing_failure)).await.unwrap();
        // log::info!("\t➡️ received an error whilst fuzzing: {}\ndescription: ({}) {}\nurl: {}", err, fuzzed_url.id, fuzzed_url.description, fuzzed_url.url);
        // let mut metrics = self.metrics.write().await;
        // metrics.failed_requests += 1;
        // metrics.total_requests += 1;
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
  }
}

#[cfg(test)]
mod tests {
  use hyper::http;
  use crate::domain::input::QueryParameter;
  use super::*;

  struct MockClient;

  #[async_trait]
  impl HttpClient for MockClient {
    async fn get(&self, _url: &str) -> Result<Response, Error> {
      Ok(Response::from(http::response::Response::new("test")))
    }
  }

  #[tokio::test]
  async fn fuzzed_url_creation() {
    let url = FuzzedUrl::new("https://example.com".into(), "desc".into(), "id".into());
    assert_eq!(url.url(), "https://example.com");
    assert_eq!(url.description(), "desc");
    assert_eq!(url.id(), "id");
  }

  #[test]
  fn fuzzing_failure_equality() {
    let failure_1 = FuzzingFailure::new(None, Some(404), None);
    let failure_2 = FuzzingFailure::new(None, Some(404), None);
    let failure_3 = FuzzingFailure::new(None, Some(500), None);

    assert_eq!(failure_1, failure_2);
    assert_ne!(failure_1, failure_3);
  }

  #[test]
  fn test_generate_url() {
    let (tx, _rx) = tokio::sync::mpsc::channel::<FuzzingResult>(1);
    let fuzzer = Fuzzer::new(1, tx);
    let input_data = FuzzyInput::new("https://example.com".into(), "/base/path".into(), vec![]);
    let path = Path::new(
      "/test".into(),
      HttpMethod::GET,
      200,
      vec![],
      "".into(),
      vec![],
      vec![],
      "".into()
    );

    let generated_url = fuzzer.generate_url(&input_data, &path);
    assert!(generated_url.is_ok());

    let actual_generated_url = generated_url.unwrap();
    assert!(actual_generated_url.has_host());
    assert_eq!(actual_generated_url.as_str(), "https://example.com/base/path/test?");
  }

  #[test]
  fn test_generate_url_with_query_params() {
    let (tx, _rx) = tokio::sync::mpsc::channel::<FuzzingResult>(1);
    let fuzzer = Fuzzer::new(1, tx);
    let input_data = FuzzyInput::new("https://example.com".into(), "/base/path".into(), vec![]);
    let query_param_1 = QueryParameter::new("q1".into(), false, Option::Some("val1".into()));
    let query_param_2 = QueryParameter::new("q2".into(), false, Option::Some("val2".into()));
    let path = Path::new(
      "/test".into(),
      HttpMethod::GET,
      200,
      vec![],
      "".into(),
      vec![query_param_1, query_param_2],
      vec![],
      "".into()
    );

    let generated_url = fuzzer.generate_url(&input_data, &path);
    assert!(generated_url.is_ok());

    let actual_generated_url = generated_url.unwrap();
    assert!(actual_generated_url.has_host());
    assert_eq!(actual_generated_url.as_str(), "https://example.com/base/path/test?q1=val1&q2=val2");
  }

  #[test]
  fn test_generate_url_with_fuzzed_query_param_values() {
    let (tx, _rx) = tokio::sync::mpsc::channel::<FuzzingResult>(1);
    let fuzzer = Fuzzer::new(1, tx);
    let input_data = FuzzyInput::new("https://example.com".into(), "/base/path".into(), vec![]);
    let query_param_1 = QueryParameter::new("q1".into(), false, Option::Some("val1".into()));
    let query_param_2 = QueryParameter::new("q2".into(), true, Option::None);
    let path = Path::new(
      "/test".into(),
      HttpMethod::GET,
      200,
      vec![],
      "".into(),
      vec![query_param_1, query_param_2],
      vec![],
      "".into()
    );

    let generated_url = fuzzer.generate_url(&input_data, &path);
    assert!(generated_url.is_ok());

    let actual_generated_url = generated_url.unwrap();
    assert!(actual_generated_url.has_host());
    assert_eq!(actual_generated_url.as_str(), "https://example.com/base/path/test?q1=val1&q2=%7Bfuzz%7D");
  }
}
