# Generate XDR for the JSON files within this directory.
generate-xdr:
	stellar-xdr encode --type ContractEvent --output stream \
		event_transfer.json \
		event_transfer_data_map.json \
		event_approve.json \
		> events.xdr
	stellar-xdr encode --type ScSpecEntry --output stream \
		spec_transfer.json \
		spec_approve.json \
		> spec.xdr
