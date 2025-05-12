use std::fs::File;
use std::path::PathBuf;
use std::collections::HashMap;

use clap::Parser;
use stellar_xdr::curr::{
    ContractEvent, ContractEventBody, ScSpecEntry, ScSpecEventDataFormat, ScSpecEventParamLocationV0, ScSpecEventParamV0, ScSpecTypeDef, ScVal
};
use serde_json::{json, Value as JsonValue};

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
            
            // Generate the derived JSON object
            let derived_json = generate_derived_json(&event, spec_entry);
            
            // Output the derived JSON
            println!("\nDerived JSON:");
            let formatted_json = serde_json::to_string_pretty(&derived_json)?;
            println!("{}", formatted_json);
            
            found_match = true;
            break;  // Use the first matching spec
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
            ScVal::String(event_topic_str) => {
                if event_topic_str.to_string() != spec_prefix_sym.to_string() {
                    return false; // Symbol/String names don't match
                }
            },
            _ => return false, // Event topic at this position is not a Symbol/String
        }
    }

    // Separate spec params by location
    let mut topic_params: Vec<&ScSpecEventParamV0> = Vec::new();
    let mut data_params: Vec<&ScSpecEventParamV0> = Vec::new();

    for param in spec.params.iter() {
        if param.location == ScSpecEventParamLocationV0::TopicList {
            topic_params.push(param);
        } else if param.location == ScSpecEventParamLocationV0::Data {
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

            // Create a map of event entries
            let mut event_map: HashMap<String, &ScVal> = HashMap::new();
            for entry in map_entries.iter() {
                // Key must be a Symbol
                let key_name = match &entry.key {
                    ScVal::Symbol(s) => s.to_string(),
                    _ => return false, // Map key is not a Symbol
                };
                event_map.insert(key_name, &entry.val);
            }

            // Check if all required keys are present and have matching types
            let mut matching_keys = 0;
            for (key, expected_type) in &data_param_spec_map {
                if let Some(value) = event_map.get(key) {
                    if sc_val_matches_spec_type(value, expected_type) {
                        matching_keys += 1;
                    } else {
                        return false; // Type mismatch for key
                    }
                } else {
                    // Key not found in event, check if it's an Option type (optional parameter)
                    if let ScSpecTypeDef::Option(_) = expected_type {
                        // Optional parameter is allowed to be missing
                        matching_keys += 1;
                    }
                    // For non-Option types, missing keys are allowed in our lenient mapping approach
                }
            }

            // We consider it a match if we have at least one matching key
            if matching_keys == 0 {
                return false;
            }
        },
        ScSpecEventDataFormat::Vec => {
            // Event data must be a Vec variant containing Some(ScVec)
            let vec_entries = match &event_body.data {
                ScVal::Vec(Some(sc_vec)) => sc_vec,
                _ => return false, // Event data is not a vec or is None
            };

            // Check that we have at least some elements to match
            if vec_entries.is_empty() || data_params.is_empty() {
                return false;
            }

            // We'll be lenient with size mismatches, but we still need data
            let matching_count = std::cmp::min(vec_entries.len(), data_params.len());
            
            // Check the types of the elements we have
            for i in 0..matching_count {
                if !sc_val_matches_spec_type(&vec_entries[i], &data_params[i].type_) {
                    return false; // Type mismatch for an element in the vec
                }
            }
            
            // We require at least one matching element
            if matching_count == 0 {
                return false;
            }
        },
    }

    // If all checks passed, the event matches the spec
    true
}

// Function to generate a self-describing JSON from event data using spec
fn generate_derived_json(event: &ContractEvent, spec_entry: &ScSpecEntry) -> JsonValue {
    // Extract event body and spec
    let ContractEventBody::V0(event_body) = &event.body;
    let spec = if let ScSpecEntry::EventV0(spec) = spec_entry {
        spec
    } else {
        return json!({ "error": "Spec is not an EventV0 variant" });
    };

    // Initialize the result object
    let mut result = serde_json::Map::new();

    // Add basic information
    result.insert("event_type".to_string(), json!(spec.name.to_string()));
    result.insert("contract_id".to_string(), json!(format!("{:?}", event.contract_id)));

    // Access the event data
    let topics = &event_body.topics;
    let skip_topics = spec.prefix_topics.len();
    
    // Create a map for looking up map data entries by key
    let mut map_data_entries: HashMap<String, &ScVal> = HashMap::new();
    if let ScVal::Map(Some(map_entries)) = &event_body.data {
        for entry in map_entries.iter() {
            if let ScVal::Symbol(key) = &entry.key {
                map_data_entries.insert(key.to_string(), &entry.val);
            }
        }
    }
    
    // Extract vec data entries if available
    let mut vec_data_entries: Vec<&ScVal> = Vec::new();
    if let ScVal::Vec(Some(vec_entries)) = &event_body.data {
        vec_data_entries = vec_entries.iter().collect();
    }
    
    // Initialize the flattened parameters
    let mut params = serde_json::Map::new();
    
    // Count data parameters we've processed so far (for vec indexing)
    let mut topic_param_count = 0;
    let mut data_param_count = 0;
    
    // Build parameters in spec-defined order by iterating through the spec params
    for param in spec.params.iter() {
        let param_name = param.name.to_string();
        
        if param.location == ScSpecEventParamLocationV0::TopicList {
            // Topic parameter - get from topics list (after prefix topics)
            let topic_index = skip_topics + topic_param_count;
            if topic_index < topics.len() {
                params.insert(
                    param_name,
                    serde_json::to_value(&topics[topic_index]).unwrap(),
                );
                topic_param_count += 1;
            }
        } else if param.location == ScSpecEventParamLocationV0::Data {
            // Data parameter - handle based on data format
            match spec.data_format {
                ScSpecEventDataFormat::SingleValue => {
                    params.insert(
                        param_name,
                        serde_json::to_value(&event_body.data).unwrap(),
                    );
                },
                ScSpecEventDataFormat::Map => {
                    if let Some(val) = map_data_entries.get(&param_name) {
                        params.insert(
                            param_name,
                            serde_json::to_value(val).unwrap(),
                        );
                    }
                },
                ScSpecEventDataFormat::Vec => {
                    if data_param_count < vec_data_entries.len() {
                        params.insert(
                            param_name,
                            serde_json::to_value(vec_data_entries[data_param_count]).unwrap(),
                        );
                        data_param_count += 1;
                    }
                },
            }
        }
    }
    
    // Include any additional data entries not explicitly defined in the spec
    // (Only for Map format, as we want to be lenient in matching)
    if spec.data_format == ScSpecEventDataFormat::Map {
        for (key, val) in &map_data_entries {
            if !params.contains_key(key) {
                params.insert(
                    key.clone(),
                    serde_json::to_value(val).unwrap(),
                );
            }
        }
    }
    
    // Add the flattened parameters to the result
    result.insert("params".to_string(), JsonValue::Object(params));

    JsonValue::Object(result)
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
