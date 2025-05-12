use std::fs;
use std::path::PathBuf;
// std imports

use clap::Parser;
use serde::Deserialize;
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
    type_: serde_json::Value, // Changed from String to Value to handle complex types
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

/// Finds a spec that matches the given event by checking if the first topic in the event
/// matches any of the specs' prefix_topics. Additional matching criteria include ensuring
/// that there are enough topics in the event to satisfy the spec's parameters.
///
/// Returns Some(&Spec) if a matching spec is found, None otherwise.
fn find_matching_spec<'a>(event: &Event, specs: &'a [Spec]) -> Option<&'a Spec> {
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

/// Processes an event according to a matching spec to produce a self-describing output.
/// This function extracts parameter values from the event based on their location (topic_list or data)
/// and data format (single_value, vec, or map) and structures them according to the spec's definition.
///
/// Returns a JSON Value containing the processed event in a self-describing format.
fn process_event(event: &Event, spec: &Spec) -> Value {
    // Create the top-level output structure
    let mut params = Vec::new();
    let mut topic_index = 1; // Skip first topic which is the event name
    let mut data_index = 0;  // Index for "vec" data format

    // Process each parameter from the spec
    for param in &spec.event_v0.params {
        let mut param_value = Value::Null;
        // We'll construct the type string for output later

        // We don't need to extract the type string here since we'll do it later
        // when creating the descriptive_type

        match param.location.as_str() {
            "topic_list" => {
                if topic_index < event.body.v0.topics.len() {
                    match &event.body.v0.topics[topic_index] {
                        EventTopic::Symbol(symbol) => {
                            param_value = json!(symbol.symbol.clone());
                        },
                        EventTopic::Address(address) => {
                            param_value = json!(address.address.clone());
                        }
                    }
                    topic_index += 1;
                }
            },
            "data" => {
                match spec.event_v0.data_format.as_str() {
                    "single_value" => {
                        // For single_value, use the entire data field
                        param_value = event.body.v0.data.clone();
                    },
                    "vec" => {
                        // For vec, extract the value at the current data_index
                        param_value = match &event.body.v0.data {
                            Value::Object(map) => {
                                map.get("vec")
                                    .and_then(|vec_data| vec_data.as_array())
                                    .and_then(|arr| {
                                        if data_index < arr.len() {
                                            let val = arr[data_index].clone();
                                            data_index += 1;
                                            Some(val)
                                        } else {
                                            None
                                        }
                                    })
                                    .unwrap_or(Value::Null)
                            },
                            _ => Value::Null
                        };
                    },
                    "map" => {
                        // For map, find the key matching param.name
                        // Extract map entries with more concise and readable code
                        param_value = match &event.body.v0.data {
                            Value::Object(map) => {
                                map.get("map")
                                    .and_then(|map_data| map_data.as_array())
                                    .map(|entries| {
                                        // Search through entries for matching name
                                        for entry in entries {
                                            let key_name = entry.get("key")
                                                .and_then(|k| k.get("symbol"))
                                                .and_then(|s| s.as_str());

                                            if let Some(name) = key_name {
                                                if name == param.name {
                                                    if let Some(val) = entry.get("val") {
                                                        return val.clone();
                                                    }
                                                }
                                            }
                                        }
                                        Value::Null
                                    })
                                    .unwrap_or(Value::Null)
                            },
                            _ => Value::Null
                        };
                    },
                    _ => {
                        // Unrecognized data format
                    }
                }
            },
            _ => {
                // Unrecognized location
            }
        }

        // Create a descriptive type string for the output
        let descriptive_type = match &param.type_ {
            Value::String(s) => s.clone(),
            Value::Object(obj) if obj.contains_key("option") => {
                // Handle option types
                obj.get("option")
                   .and_then(|opt| opt.get("value_type"))
                   .and_then(|vt| vt.as_str())
                   .map(|s| format!("option<{}>", s))
                   .unwrap_or_else(|| "option<unknown>".to_string())
            },
            _ => "unknown".to_string()
        };

        // Add parameter to params array
        params.push(json!({
            "name": param.name,
            "type": descriptive_type,
            "value": param_value
        }));
    }

    // Build final result
    json!({
        "name": spec.event_v0.name,
        "type": "event",
        "params": params
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Read event file
    let event_json = fs::read_to_string(&args.event)
        .map_err(|e| format!("Failed to read event file {}: {}", args.event.display(), e))?;
    let event: Event = serde_json::from_str(&event_json)
        .map_err(|e| format!("Failed to parse event JSON: {}", e))?;

    // Read all spec files
    let mut specs = Vec::new();
    for spec_path in &args.specs {
        let spec_json = fs::read_to_string(spec_path)
            .map_err(|e| format!("Failed to read spec file {}: {}", spec_path.display(), e))?;
        let spec: Spec = serde_json::from_str(&spec_json)
            .map_err(|e| format!("Failed to parse spec JSON from {}: {}", spec_path.display(), e))?;
        specs.push(spec);
    }

    if specs.is_empty() {
        eprintln!("No spec files were provided or all spec files were invalid.");
        std::process::exit(1);
    }

    // Find matching spec
    if let Some(matching_spec) = find_matching_spec(&event, &specs) {
        // Process event with matching spec
        let result = process_event(&event, matching_spec);

        // Output the result
        println!("{}", serde_json::to_string_pretty(&result)
            .map_err(|e| format!("Failed to serialize result to JSON: {}", e))?);
        Ok(())
    } else {
        eprintln!("No matching spec found for the provided event.");
        std::process::exit(1);
    }
}
