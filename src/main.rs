use std::fs;
use std::path::PathBuf;
use std::collections::HashMap;

use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to event JSON file
    #[arg(short, long)]
    event: PathBuf,

    /// Paths to spec JSON files
    #[arg(short, long, num_args = 1..)]
    specs: Vec<PathBuf>,
}

#[derive(Deserialize, Debug)]
struct EventTopicSymbol {
    symbol: String,
}

#[derive(Deserialize, Debug)]
struct EventTopicAddress {
    address: String,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum EventTopic {
    Symbol(EventTopicSymbol),
    Address(EventTopicAddress),
}

#[derive(Deserialize, Debug)]
struct EventV0 {
    topics: Vec<EventTopic>,
    data: Value,
}

#[derive(Deserialize, Debug)]
struct EventBody {
    v0: EventV0,
}

#[derive(Deserialize, Debug)]
struct Event {
    ext: String,
    contract_id: String,
    #[serde(rename = "type_")]
    type_: String,
    body: EventBody,
}

#[derive(Deserialize, Debug)]
struct SpecParam {
    doc: String,
    name: String,
    #[serde(rename = "type_")]
    type_: String,
    location: String,
}

#[derive(Deserialize, Debug)]
struct SpecEventV0 {
    doc: String,
    lib: String,
    name: String,
    prefix_topics: Vec<String>,
    params: Vec<SpecParam>,
    data_format: String,
}

#[derive(Deserialize, Debug)]
struct Spec {
    event_v0: SpecEventV0,
}

fn find_matching_spec(event: &Event, specs: &[Spec]) -> Option<&Spec> {
    // First, extract the first topic (event name) from the event
    if let Some(EventTopic::Symbol(symbol)) = event.body.v0.topics.first() {
        let event_name = &symbol.symbol;

        // Find a spec with matching name in prefix_topics
        for spec in specs {
            if spec.event_v0.prefix_topics.contains(event_name) &&
               event.body.v0.topics.len() - 1 >= spec.event_v0.params.iter().filter(|p| p.location == "topic_list").count() {
                return Some(spec);
            }
        }
    }

    None
}

fn process_event(event: &Event, spec: &Spec) -> Value {
    let mut result = json!({
        "contract_id": event.contract_id,
        "name": spec.event_v0.name,
    });

    // Process parameters from spec
    let mut param_index = 0;
    let mut topic_index = 1; // Skip first topic which is the event name

    for param in &spec.event_v0.params {
        match param.location.as_str() {
            "topic_list" => {
                if topic_index < event.body.v0.topics.len() {
                    match &event.body.v0.topics[topic_index] {
                        EventTopic::Symbol(symbol) => {
                            result[param.name.clone()] = json!({
                                "type": param.type_.clone(),
                                "value": symbol.symbol.clone()
                            });
                        },
                        EventTopic::Address(address) => {
                            result[param.name.clone()] = json!({
                                "type": param.type_.clone(),
                                "value": address.address.clone()
                            });
                        }
                    }
                    topic_index += 1;
                }
            },
            "data" => {
                if spec.event_v0.data_format == "single_value" {
                    result[param.name.clone()] = json!({
                        "type": param.type_.clone(),
                        "value": event.body.v0.data
                    });
                } else if spec.event_v0.data_format == "map" {
                    // Handle map data format if needed
                    if let Value::Object(map) = &event.body.v0.data {
                        for (name, value) in map {
                            result[name.clone()] = json!({
                                "type": param.type_.clone(),
                                "value": value
                            });
                        }
                    }
                }
                param_index += 1;
            },
            _ => {
                // Unrecognized location
            }
        }
    }

    result
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Read event file
    let event_json = fs::read_to_string(&args.event)?;
    let event: Event = serde_json::from_str(&event_json)?;

    // Read all spec files
    let mut specs = Vec::new();
    for spec_path in &args.specs {
        let spec_json = fs::read_to_string(spec_path)?;
        let spec: Spec = serde_json::from_str(&spec_json)?;
        specs.push(spec);
    }

    // Find matching spec
    if let Some(matching_spec) = find_matching_spec(&event, &specs) {
        // Process event with matching spec
        let result = process_event(&event, matching_spec);

        // Output the result
        println!("{}", serde_json::to_string_pretty(&result)?);
        Ok(())
    } else {
        eprintln!("No matching spec found for the provided event.");
        std::process::exit(1);
    }
}
