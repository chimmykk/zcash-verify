#!/bin/bash
echo "Generating basic Zcash ownership proof (no social identities attached)..."

cargo run -p zcash-verifier -- prove orchard \
  --seed "<seed phrase >" \
  --network main \
  --start-height 3295000 \
  --output zcash_prove.json

echo "Saved simple proof to zcash_prove.json"
