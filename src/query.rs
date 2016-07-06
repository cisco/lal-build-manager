use util::artifactory::get_latest_versions;
use super::{LalResult, Config};

pub fn query(cfg: &Config, component: &str) -> LalResult<()> {
  let vers = try!(get_latest_versions(&cfg.artifactory, component));
  for v in vers {
    println!("{}", v);
  }
  Ok(())
}
