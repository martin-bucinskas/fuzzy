use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct CliFuzzEndpoint {
  #[structopt(short)]
  pub(crate) input_file: String,
}

impl CliFuzzEndpoint {
  pub fn parse() -> Self {
    CliFuzzEndpoint::from_args()
  }
}
