use std::fs::File;
use std::path::PathBuf;
use std::collections::HashMap;

use clap::Parser;
use stellar_xdr::curr::{
    ContractEvent, ContractEventBody, ScSpecEntry, ScSpecTypeDef, ScVal,
    ScSpecEventParamV0, ScSpecEventDataFormat,
};

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
    
    let mut found_match = false;
    for (i, spec_entry) in specs.iter().enumerate() {
        if event_matches_spec(&event, spec_entry) {
            // Found a match!
            let spec_path = &args.specs[i];
            println!("Found matching spec: {}", spec_path.display());
            found_match = true;
        }
    }

    if !found_match {
        println!("No matching spec found for the event.");
    }

    Ok(())
}

// Helper function to check if an ScVal's type matches a ScSpecTypeDef
fn sc_val_matches_spec_type(val: &ScVal, spec_type: &ScSpecTypeDef) -> bool {
    match (val, spec_type) {
        // Simple scalar types
        (ScVal::Bool(_), ScSpecTypeDef::Bool) => true,
        (ScVal::Void, ScSpecTypeDef::Void) => true,
        (ScVal::Error(_), ScSpecTypeDef::Error) => true,
        (ScVal::U32(_), ScSpecTypeDef::U32) => true,
        (ScVal::I32(_), ScSpecTypeDef::I32) => true,
        (ScVal::U64(_), ScSpecTypeDef::U64) => true,
        (ScVal::I64(_), ScSpecTypeDef::I64) => true,
        (ScVal::U128(_), ScSpecTypeDef::U128) => true,
        (ScVal::I128(_), ScSpecTypeDef::I128) => true,
        (ScVal::U256(_), ScSpecTypeDef::U256) => true,
        (ScVal::I256(_), ScSpecTypeDef::I256) => true,
        (ScVal::Address(_), ScSpecTypeDef::Address) => true,
        (ScVal::Symbol(_), ScSpecTypeDef::Symbol) => true,
        (ScVal::String(_), ScSpecTypeDef::String) => true,
        (ScVal::Bytes(_), ScSpecTypeDef::Bytes) => true,

        // Container types (check outer type only)
        (ScVal::Vec(Some(_)), ScSpecTypeDef::Vec(_)) => true,
        (ScVal::Map(Some(_)), ScSpecTypeDef::Map(_)) => true,

        // Option type
        (_, ScSpecTypeDef::Option(option_spec)) => {
            sc_val_matches_spec_type(val, &option_spec.value_type)
        },

        // BytesN type
        (ScVal::Bytes(bytes), ScSpecTypeDef::BytesN(bytes_n_spec)) => {
            bytes.len() == bytes_n_spec.n as usize
        },

        // Any other combination is a mismatch
        _ => false,
    }
}

fn event_matches_spec(event: &ContractEvent, spec_entry: &ScSpecEntry) -> bool {
    // Extract the V0 variant from the event body
    let ContractEventBody::V0(event_body) = &event.body;

    // Extract the EventV0 variant from the spec
    let spec = if let ScSpecEntry::EventV0(spec) = spec_entry {
        spec
    } else {
        return false; // Not an EventV0 spec or different version
    };

    // Check prefix topics
    let prefix_topics = &spec.prefix_topics;
    let topics = &event_body.topics;

    // Ensure there are enough topics in the event for the prefix
    if topics.len() < prefix_topics.len() {
        return false;
    }

    // Check each prefix topic matches the corresponding event topic
    for (i, spec_prefix_sym) in prefix_topics.iter().enumerate() {
        match &topics[i] {
            ScVal::Symbol(event_topic_sym) => {
                if event_topic_sym.to_string() != spec_prefix_sym.to_string() {
                    return false; // Symbol names don't match
                }
            },
            _ => return false, // Event topic at this position is not a Symbol
        }
    }

    // Separate spec params by location
    let mut topic_params: Vec<&ScSpecEventParamV0> = Vec::new();
    let mut data_params: Vec<&ScSpecEventParamV0> = Vec::new();

    for param in spec.params.iter() {
        if param.location == stellar_xdr::curr::ScSpecEventParamLocationV0::TopicList {
            topic_params.push(param);
        } else if param.location == stellar_xdr::curr::ScSpecEventParamLocationV0::Data {
            data_params.push(param);
        } else {
            return false; // Unknown location
        }
    }

    // Check topic params
    let expected_topic_count = prefix_topics.len() + topic_params.len();
    if topics.len() != expected_topic_count {
        return false; // Wrong number of total topics
    }

    // Check the type of each topic param against the event topics after the prefix
    for (i, param) in topic_params.iter().enumerate() {
        let topic_index = prefix_topics.len() + i;
        let event_topic_val = &topics[topic_index];

        if !sc_val_matches_spec_type(event_topic_val, &param.type_) {
            return false; // Type mismatch for a topic parameter
        }
    }

    // Check data params based on spec's data format
    match spec.data_format {
        ScSpecEventDataFormat::SingleValue => {
            // Expect exactly one data parameter defined in the spec
            if data_params.len() != 1 {
                return false; // Spec error: SingleValue format requires 1 data param
            }
            let param = data_params[0]; // Get the single expected data parameter

            // Check if the event's data field matches the type specified by the param
            if !sc_val_matches_spec_type(&event_body.data, &param.type_) {
                return false; // Type mismatch for single data value
            }
        },
        ScSpecEventDataFormat::Map => {
            // Event data must be a Map variant containing Some(ScMap)
            let map_entries = match &event_body.data {
                ScVal::Map(Some(sc_map)) => sc_map,
                _ => return false, // Event data is not a map or is None
            };

            // Build a lookup map from the spec's data parameters (name -> type)
            let data_param_spec_map: HashMap<String, &ScSpecTypeDef> = data_params
                .iter()
                .map(|p| (p.name.to_string(), &p.type_))
                .collect();

            // Event map must have the same number of entries as data params specified
            if map_entries.len() != data_param_spec_map.len() {
                return false; // Mismatch in number of map entries vs specified data params
            }

            // Check each entry in the event's data map
            for entry in map_entries.iter() {
                // Key must be a Symbol
                let key_name = match &entry.key {
                    ScVal::Symbol(s) => s.to_string(),
                    _ => return false, // Map key is not a Symbol
                };

                // Look up the key in the spec parameter map
                match data_param_spec_map.get(&key_name) {
                    Some(expected_type) => {
                        // Check if the value type matches
                        if !sc_val_matches_spec_type(&entry.val, expected_type) {
                            return false; // Value type mismatch for key
                        }
                    },
                    None => return false, // Key from event map was not found in spec
                }
            }
        },
        ScSpecEventDataFormat::Vec => {
            // Event data must be a Vec variant containing Some(ScVec)
            let vec_entries = match &event_body.data {
                ScVal::Vec(Some(sc_vec)) => sc_vec,
                _ => return false, // Event data is not a vec or is None
            };

            // Event vec must have the same number of elements as data params specified
            if vec_entries.len() != data_params.len() {
                return false; // Mismatch in number of vec elements vs specified data params
            }

            // Check each element type against the corresponding data parameter type
            for (event_val, spec_param) in vec_entries.iter().zip(data_params.iter()) {
                if !sc_val_matches_spec_type(event_val, &spec_param.type_) {
                    return false; // Type mismatch for an element in the vec
                }
            }
        },
    }

    // If all checks passed, the event matches the spec
    true
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