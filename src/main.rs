use std::path::PathBuf;
use std::fs::File;

use clap::Parser;
use stellar_xdr::curr::{
    ContractEvent, ScSpecEntry,
};

mod param_types;
use param_types::ParamTypes;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    event: PathBuf,

    #[arg(long = "spec")]
    specs: Vec<PathBuf>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let event = args.event()?;
    let specs = args.specs()?;

    let event_types = event.param_types();
    dbg!(&event_types);
    for s in specs {
        let s_types = s.param_types();
        dbg!(&s_types);
        if event_types == s_types {
            dbg!("match");
        }
    }

    Ok(())
}

impl Args {
    fn event(&self) -> Result<ContractEvent, Box<dyn std::error::Error>> {
        Ok(serde_json::from_reader::<_, ContractEvent>(File::open(
            &self.event,
        )?)?)
    }

    fn specs(&self) -> Result<Vec<ScSpecEntry>, Box<dyn std::error::Error>> {
        Ok(self
            .specs
            .iter()
            .map(|p| File::open(p))
            .collect::<Result<Vec<_>, _>>()?
            .iter()
            .map(|f| serde_json::from_reader::<_, ScSpecEntry>(f))
            .collect::<Result<Vec<_>, _>>()?)
    }
}
