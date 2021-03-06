use std::fs;
use std::io::{Read, Result as IoResult};
use std::path::PathBuf;
use std::collections::HashMap;

use structopt::StructOpt;
use quicli::prelude::*;

#[derive(Debug, StructOpt)]
struct Cli {
  #[structopt(flatten)]
  verbosity: Verbosity,
  #[structopt(help="envvar name")]
  envvar: String,
}

#[derive(Eq, PartialEq, Hash, Debug)]
enum EnvVal {
  Value(String),
  Nothing,
  Fail,
}

fn get_envval(mut path: PathBuf, name: &str) -> IoResult<Option<String>> {
  path.push("environ");
  let mut buffer = vec![];
  let mut f = fs::File::open(&path)?;
  f.read_to_end(&mut buffer)?;
  let r = buffer.split(|c| *c == 0)
    .find(|v| v.starts_with(name.as_bytes()))
    .and_then(|v| {
      v.splitn(2, |c| *c == b'=')
        .nth(1)
        .map(|s| String::from_utf8_lossy(s).into_owned())
    });
  Ok(r)
}

fn main() -> CliResult {
  let args = Cli::from_args();
  let name_prefix = args.envvar + "=";
  let result: Vec<(EnvVal, u32)> = fs::read_dir("/proc")?
    .collect::<Vec<_>>()
    .par_iter().filter_map(|entry| {
      match *entry {
        Ok(ref entry) => {
          let path = entry.path();
          if let Ok(pid) = path.file_name().unwrap().to_str().unwrap().parse() {
            Some((path, pid))
          } else {
            None
          }
        },
        Err(_) => None,
      }
    }).map(|(path, pid)| {
      let v = get_envval(path, &name_prefix);
      let v = match v {
        Ok(Some(s)) => EnvVal::Value(s),
        Ok(None) => EnvVal::Nothing,
        Err(_) => EnvVal::Fail,
      };
      (v, pid)
    }).collect();

  let mut map = HashMap::new();
  for (v, pid) in result {
    map.entry(v).or_insert_with(|| vec![]).push(pid);
  }

  let mut r = map.into_iter().collect::<Vec<(EnvVal, Vec<u32>)>>();
  r.sort_unstable_by_key(|&(_, ref pids)| pids.len());

  for (v, pids) in r {
    println!("{:5} {:?} ({:?})", pids.len(), v, pids);
  }

  Ok(())
}
