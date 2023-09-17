use serde::Deserialize;
use url::{ParseError, Url};

#[derive(Clone, Debug, Deserialize)]
pub struct FuzzyInput {
  host: String,
  base_path: String,
  paths: Vec<Path>,
}

impl FuzzyInput {

  pub fn new(host: String, base_path: String, paths: Vec<Path>) -> Self {
    Self {
      host,
      base_path,
      paths,
    }
  }

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

  pub fn new(endpoint: String, method: HttpMethod, expected_status: u16, expected_headers: Vec<ExpectedHeader>, expected_body: String, query_parameters: Vec<QueryParameter>, headers: Vec<HeaderParameter>, body: String) -> Self {
    Self {
      endpoint,
      method,
      expected_status,
      expected_headers,
      expected_body,
      query_parameters,
      headers,
      body,
    }
  }
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
      base_path.trim_start_matches('/').trim_end_matches('/'),
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

  pub fn new(name: String, fuzz: bool, value: Option<String>) -> Self {
    Self { name, fuzz, value }
  }
  
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

#[cfg(test)]
mod tests {
  use super::*;
  use serde_yaml;

  #[test]
  fn test_fuzzy_input_deserialization() {
    let data = r#"
        host: "http://example.com"
        base_path: "/api/v1"
        paths: []
        "#;

    let fuzzy_input: FuzzyInput = serde_yaml::from_str(data).unwrap();
    assert_eq!(fuzzy_input.host(), "http://example.com");
    assert_eq!(fuzzy_input.base_path(), "/api/v1");
  }

  #[test]
  fn test_path_deserialization() {
    let data = r#"
        endpoint: "/test"
        method: GET
        expected_status: 200
        expected_headers: []
        expected_body: ""
        query_parameters: []
        headers: []
        body: ""
        "#;

    let path: Path = serde_yaml::from_str(data).unwrap();
    assert_eq!(path.endpoint(), "/test");
    assert_eq!(path.method(), &HttpMethod::GET);
  }

  #[test]
  fn test_expected_header_deserialization() {
    let data = r#"
        name: "Content-Type"
        value: "application/json"
        "#;

    let expected_header: ExpectedHeader = serde_yaml::from_str(data).unwrap();
    assert_eq!(expected_header.name(), "Content-Type");
    assert_eq!(expected_header.value(), "application/json");
  }

  #[test]
  fn test_header_parameter_deserialization() {
    let data = r#"
        name: "Accept"
        value: "application/json"
        fuzz: false
        "#;

    let header_param: HeaderParameter = serde_yaml::from_str(data).unwrap();
    assert_eq!(header_param.name(), "Accept");
    assert_eq!(header_param.value(), &Some("application/json".to_string()));
  }

  #[test]
  fn test_query_parameter_deserialization() {
    let data = r#"
        name: "test_param"
        fuzz: true
        "#;

    let query_param: QueryParameter = serde_yaml::from_str(data).unwrap();
    assert_eq!(query_param.name(), "test_param");
    assert_eq!(query_param.fuzz(), true);
  }

  #[test]
  fn test_path_to_url() {
    let path = Path {
      endpoint: "/test_endpoint".to_string(),
      method: HttpMethod::GET,
      expected_status: 200,
      expected_headers: vec![],
      expected_body: "".to_string(),
      query_parameters: vec![
        QueryParameter {
          name: "test".to_string(),
          fuzz: true,
          value: None,
        },
        QueryParameter {
          name: "key".to_string(),
          fuzz: false,
          value: Some("value".to_string()),
        },
      ],
      headers: vec![],
      body: "".to_string(),
    };

    let base_host = &"http://example.com".to_string();
    let base_path = &"/api/v1".to_string();
    let url = path.to_url(base_host, base_path).unwrap();
    assert_eq!(url.as_str(), "http://example.com/api/v1/test_endpoint?test=%7Bfuzz%7D&key=value");
  }
}
