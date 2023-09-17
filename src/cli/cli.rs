use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct CliFuzzEndpoint {
  #[structopt(short)]
  pub(crate) input_file: String,
}

impl CliFuzzEndpoint {
  pub fn from_slice(args: &[&str]) -> Result<Self, structopt::clap::Error> {
    CliFuzzEndpoint::from_iter_safe(args)
  }

  pub fn parse() -> Self {
    CliFuzzEndpoint::from_args()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_parse_input_file() {
    let parsed = CliFuzzEndpoint::from_slice(&["fuzzy", "-i", "some_file.txt"]).unwrap();

    assert_eq!(parsed.input_file, "some_file.txt");
  }
}
