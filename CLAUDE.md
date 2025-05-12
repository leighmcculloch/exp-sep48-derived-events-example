Build a Rust CLI in the existing project that takes JSON objects like those in the 'event' list below, and marries them with the JSON objects in the 'spec' list below. The spec describes what each field is in the event.

Output a derived JSON object that contains the data in the 'event', structured in such a way that it is self describing based on the 'spec'.

The events and specs are 1-1 in the examples listed below, but in reality will not be. Map through trial and error, mapping an event to a spec if it just happens to match.

event list:
- testfixtures/event_approve.json
- testfixtures/event_transfer.json
- testfixtures/event_transfer_data_map.json

spec list:
- testfixtures/spec_approve.json
- testfixtures/spec_transfer.json
- testfixtures/spec_transfer_data_map.json
