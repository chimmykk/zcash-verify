#!/bin/bash
echo "Generating Zcash ownership proof and registering social identities..."

cargo run -p zcash-verifier -- register \
  --seed "<seed phrase>" \
  --x username \
  --zcashforum username \
  --bluesky username \
  --start-height 3295000 \
  --output-dir .

echo "Saved combined social proof to zcashprovewithsocial.json"
