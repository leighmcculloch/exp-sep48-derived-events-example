# SEP-48 Contract Interface Events: Example Derived Events Producer

A utility for parsing and matching Stellar Contract Events against SEP-48 Contract Interface Event specifications, producing self-describing JSON output.

This code base is only an example, and doesn't cover all scenarios or all use cases. It shows broadly what's possible with the introduction of events into SEP-49 Contract Interface Specifications, as currently being discussed in [stellar#1724](https://github.com/orgs/stellar/discussions/1724).

> [!NOTE]
> This tool uses unreleased XDR.

## Overview

This tool:

1. Takes in Stellar contract event data and event specification files, all in XDR-JSON
2. Matching events against compatible specifications
3. Transforming the event data into a human-readable, self-describing JSON format

## Installation

```
git clone https://github.com/leighmcculloch/exp-sep48-derived-events-example
cd exp-sep48-derived-events-example
cargo install --locked --path .
```

## Usage

```
stellar-events \
  --event <EVENT_JSON_PATH> \
  --spec <SPEC_JSON_PATH> \
  [--spec <ADDITIONAL_SPEC_JSON_PATH> ...]
```

### Arguments

- `--event <PATH>`: Path to an XDR-JSON file containing the contract event data
- `--spec <PATH>`: Path to a XDR-JSON file containing an event specification (can be provided multiple times)

### Example

#### Command to Run

```
$ stellar-events \
    --event testfixtures/event_approve.json \
    --spec testfixtures/spec_transfer.json \
    --spec testfixtures/spec_approve.json \
    --spec testfixtures/spec_transfer_data_map.json
```

#### Input Event

```json
{
  "ext": "v0",
  "contract_id": "CBUZJXHZ6PBS2YR3SEJZ3CIGMQBYP6367D3KQAR2NB3U2I5AOWLC4DU2",
  "type_": "contract",
  "body": {
    "v0": {
      "topics": [
        { "symbol": "approve" },
        { "address": "GBMBVAHBE6D4AJXJJVTBQTVU4G7SN4FEIJOL5YTOHZ4WCUMKQ52ANL2B" },
        { "address": "GCAN5IE4PWMWSMFZCYQRCJM73MCPG5JD2R7WF5HIFHHHDBWSDJFIWX7X" }
      ],
      "data": {
        "vec": [
          { "i128": { "hi": 0, "lo": 10000 } },
          { "u32": 1998742 }
        ]
      }
    }
  }
}
```

#### Input Matching SEP-48 Spec (`testfixtures/spec_approve.json`)

```json
{
  "event_v0": {
    "doc": "",
    "lib": "",
    "name": "approve",
    "prefix_topics": ["approve"],
    "params": [
      { "doc": "", "name": "from", "type_":"address", "location": "topic_list" },
      { "doc": "", "name": "spender", "type_":"address", "location": "topic_list" },
      { "doc": "", "name": "amount", "type_":"i128", "location": "data" },
      { "doc": "", "name": "live_until_ledger", "type_":"u32", "location": "data" }
    ],
    "data_format": "vec"
  }
}
```

#### Output

```json
{
  "event_type": "approve",
  "contract_id": "CBUZJXHZ6PBS2YR3SEJZ3CIGMQBYP6367D3KQAR2NB3U2I5AOWLC4DU2",
  "params": {
    "from": {
      "address": "GBMBVAHBE6D4AJXJJVTBQTVU4G7SN4FEIJOL5YTOHZ4WCUMKQ52ANL2B"
    },
    "spender": {
      "address": "GCAN5IE4PWMWSMFZCYQRCJM73MCPG5JD2R7WF5HIFHHHDBWSDJFIWX7X"
    },
    "amount": {
      "i128": {
        "hi": 0,
        "lo": 10000
      }
    },
    "live_until_ledger": {
      "u32": 1998742
    }
  }
}
```
