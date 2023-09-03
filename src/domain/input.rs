use serde::Deserialize;
use url::{ParseError, Url};

#[derive(Clone, Debug, Deserialize)]
pub struct FuzzyInput {
  host: String,
  base_path: String,
  paths: Vec<Path>,
}

impl FuzzyInput {
  pub fn host(&self) -> &String {
    &self.host
  }

  pub fn base_path(&self) -> &String {
    &self.base_path
  }

  pub fn paths(&self) -> &Vec<Path> {
    &self.paths
  }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Path {
  endpoint: String,
  method: HttpMethod,
  expected_status: u16,
  expected_headers: Vec<ExpectedHeader>,
  expected_body: String,
  query_parameters: Vec<QueryParameter>,
  headers: Vec<HeaderParameter>,
  body: String,
}

impl Path {
  pub fn endpoint(&self) -> &String {
    &self.endpoint
  }

  pub fn method(&self) -> &HttpMethod {
    &self.method
  }

  pub fn expected_status(&self) -> &u16 {
    &self.expected_status
  }

  pub fn expected_headers(&self) -> &Vec<ExpectedHeader> {
    &self.expected_headers
  }

  pub fn expected_body(&self) -> &String {
    &self.expected_body
  }

  pub fn headers(&self) -> &Vec<HeaderParameter> {
    &self.headers
  }

  pub fn body(&self) -> &String {
    &self.body
  }

  pub fn query_parameters(&self) -> &Vec<QueryParameter> {
    &self.query_parameters
  }

  pub fn to_url(&self, base_host: &String, base_path: &String) -> Result<Url, ParseError> {
    let combined_path = format!(
      "{}/{}/{}",
      base_host.trim_end_matches('/'),
      base_path.trim_end_matches('/'),
      &self.endpoint.trim_start_matches('/')
    );

    let params_to_fuzz: Vec<(&str, &str)> = self.query_parameters()
      .iter()
      .filter_map(|param| {
        if param.fuzz() {
          Some((param.name().as_str(), "{fuzz}"))
        } else {
          param.value().as_ref().map(|value| (param.name().as_str(), value.as_str()))
        }
      })
      .collect();

    Url::parse_with_params(&combined_path, &params_to_fuzz)
  }
}

#[derive(Clone, Debug, Deserialize)]
pub struct ExpectedHeader {
  name: String,
  value: String,
}

impl ExpectedHeader {
  pub fn name(&self) -> &String {
    &self.name
  }

  pub fn value(&self) -> &String {
    &self.value
  }
}

#[derive(Clone, Debug, Deserialize)]
pub struct HeaderParameter {
  name: String,
  value: Option<String>,
  fuzz: bool,
}

impl HeaderParameter {
  pub fn name(&self) -> &String {
    &self.name
  }

  pub fn value(&self) -> &Option<String> {
    &self.value
  }

  pub fn fuzz(&self) -> bool {
    self.fuzz
  }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum HttpMethod {
  GET,
  POST,
  PUT,
  PATCH,
  DELETE,
  HEAD,
  OPTIONS,
}

#[derive(Clone, Debug, Deserialize)]
pub struct QueryParameter {
  name: String,
  fuzz: bool,
  #[serde(default)]
  value: Option<String>,
}

impl QueryParameter {
  pub fn name(&self) -> &String {
    &self.name
  }

  pub fn fuzz(&self) -> bool {
    self.fuzz
  }

  pub fn value(&self) -> &Option<String> {
    &self.value
  }
}
