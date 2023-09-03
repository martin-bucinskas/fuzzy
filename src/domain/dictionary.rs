use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct FuzzyDictionary {
  data: Vec<FuzzyData>,
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
