use std::iter::once;

use stellar_xdr::curr::{
    ContractEvent, ContractEventBody, ScSpecEntry, ScSpecEventDataFormat,
    ScSpecEventParamLocationV0, ScSpecType, ScValType,
};

pub trait ParamTypes {
    fn param_types(&self) -> Vec<ScValType>;
}

impl ParamTypes for ContractEvent {
    fn param_types(&self) -> Vec<ScValType> {
        match &self.body {
            ContractEventBody::V0(body) => body
                .topics
                .iter()
                .map(|t| t.discriminant())
                .chain(once(body.data.discriminant()))
                .collect(),
        }
    }
}

impl ParamTypes for ScSpecEntry {
    fn param_types(&self) -> Vec<ScValType> {
        match self {
            ScSpecEntry::EventV0(s) => {
                let prefix_types = s.prefix_topics.iter().map(|_| ScValType::Symbol);
                let topic_types = s.params.iter().filter_map(|p| match &p.location {
                    ScSpecEventParamLocationV0::TopicList => {
                        Some(sc_spec_type_to_sc_val_type(p.type_.discriminant()))
                    }
                    _ => None,
                });
                let data_types = match s.data_format {
                    ScSpecEventDataFormat::SingleValue => s
                        .params
                        .iter()
                        .filter_map(|p| match &p.location {
                            ScSpecEventParamLocationV0::Data => {
                                Some(sc_spec_type_to_sc_val_type(p.type_.discriminant()))
                            }
                            _ => None,
                        })
                        .next()
                        .unwrap_or(ScValType::Void),
                    ScSpecEventDataFormat::Vec => ScValType::Vec,
                    ScSpecEventDataFormat::Map => ScValType::Map,
                };
                prefix_types
                    .chain(topic_types)
                    .chain(once(data_types))
                    .collect()
            }
            _ => Vec::new(),
        }
    }
}

fn sc_spec_type_to_sc_val_type(t: ScSpecType) -> ScValType {
    match t {
        ScSpecType::Val => todo!(),
        ScSpecType::Bool => todo!(),
        ScSpecType::Void => todo!(),
        ScSpecType::Error => todo!(),
        ScSpecType::U32 => ScValType::U32,
        ScSpecType::I32 => todo!(),
        ScSpecType::U64 => ScValType::U64,
        ScSpecType::I64 => todo!(),
        ScSpecType::Timepoint => todo!(),
        ScSpecType::Duration => todo!(),
        ScSpecType::U128 => todo!(),
        ScSpecType::I128 => ScValType::I128,
        ScSpecType::U256 => todo!(),
        ScSpecType::I256 => todo!(),
        ScSpecType::Bytes => todo!(),
        ScSpecType::String => ScValType::String,
        ScSpecType::Symbol => ScValType::Symbol,
        ScSpecType::Address => ScValType::Address,
        ScSpecType::MuxedAddress => todo!(),
        ScSpecType::Option => todo!(),
        ScSpecType::Result => todo!(),
        ScSpecType::Vec => ScValType::Vec,
        ScSpecType::Map => ScValType::Map,
        ScSpecType::Tuple => todo!(),
        ScSpecType::BytesN => todo!(),
        ScSpecType::Udt => todo!(),
    }
}
