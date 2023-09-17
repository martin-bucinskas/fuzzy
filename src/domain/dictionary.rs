use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct FuzzyDictionary {
  data: Vec<FuzzyData>,
}

impl FuzzyDictionary {
  pub fn new(data: Vec<FuzzyData>) -> Self {
    Self { data }
  }
}

#[derive(Clone, Debug, Deserialize)]
pub struct FuzzyData {
  id: String,
  description: String,
  values: Vec<String>,
}

impl FuzzyDictionary {
  pub fn data(&self) -> &Vec<FuzzyData> {
    &self.data
  }
}

impl FuzzyData {
  pub fn id(&self) -> &String {
    &self.id
  }

  pub fn description(&self) -> &String {
    &self.description
  }

  pub fn values(&self) -> &Vec<String> {
    &self.values
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_yaml;

  #[test]
  fn test_deserialization() {
    let data = r#"
        data:
          - id: "1"
            description: "test description"
            values: ["value1", "value2"]
        "#;

    let dict: FuzzyDictionary = serde_yaml::from_str(data).unwrap();
    assert_eq!(dict.data().len(), 1);
    assert_eq!(dict.data()[0].id(), "1");
    assert_eq!(dict.data()[0].description(), "test description");
    assert_eq!(dict.data()[0].values().len(), 2);
  }

  #[test]
  fn test_field_access() {
    let fuzzy_data = FuzzyData {
      id: "1".to_string(),
      description: "test description".to_string(),
      values: vec!["value1".to_string(), "value2".to_string()],
    };

    assert_eq!(fuzzy_data.id(), "1");
    assert_eq!(fuzzy_data.description(), "test description");
    assert_eq!(fuzzy_data.values().len(), 2);
  }
}
