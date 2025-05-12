use std::fs::File;
use std::path::PathBuf;

use clap::Parser;
use stellar_xdr::curr::{ContractEvent, ScSpecEntry};

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

    for s in specs {
        // TODO: For each spec, check if the shape of the params in the spec match the topics and
        // the data field in the event. If they match, output a 'found' message along with the
        // spec.
        //
        // In the spec there is a list of prefix topics, and a list of params. The prefix topics
        // are the first topics that'll be in the topic list in the event. Prefix topics are always
        // ScValType::Symbol.
        //
        // The list of params can contains params that can either be in the topics or in the data
        // value. The params in the topic appear sequentially after the prefix topics and are of
        // whatever type the ScSpecTypeDef value defines it is. Not all types need supporting, but
        // support at least these types I64, U64, U32, String, Symbol, Address, Vec, and Map. All
        // those types should one-to-one map to an ScValType pretty obviously.
        //
        // The data is where extra thinking and care is required. The spec will define the data
        // type as either a single-value, a vec, or a map.
        //
        // If the data-format is a single-value, there should only be one param in the spec labeled
        // as being in the data location, and when determining if the event matches the spec, it
        // should check if the data value has a type that matches the single-value's type.
        //
        // If the data-format is a map, then for the event to match the spec, each element in the
        // map (key field) must match a data param in the spec.
        //
        // If the data-format is a vec, then for the event to match the spec, each element in the
        // vec must match a data param in the spec.
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
