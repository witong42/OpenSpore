#!/bin/bash

# Check if the substrate directory exists
SUBSTRATE_DIR="$HOME/.openspore"
if [ -d "$SUBSTRATE_DIR" ]; then
  substrate_dir_exists=true
  substrate_dir_message="Substrate directory exists at $SUBSTRATE_DIR."
else
  substrate_dir_exists=false
  substrate_dir_message="Substrate directory does NOT exist at $SUBSTRATE_DIR!"
fi

# Check if openspore is in the PATH
if which openspore >/dev/null 2>&1; then
  openspore_in_path=true
  openspore_path_message="openspore is in the PATH."
else
  openspore_in_path=false
  openspore_path_message="openspore is NOT in the PATH!"
fi

# Determine overall status
if [ "$substrate_dir_exists" = "true" ] && [ "$openspore_in_path" = "true" ]; then
  status="OK"
  message="All health checks passed."
else
  status="ERROR"
  message="One or more health checks failed! Substrate: $substrate_dir_message, OpenSpore: $openspore_path_message"
fi

# Output JSON
cat <<EOF
{
  "substrate_dir_exists": ${substrate_dir_exists},
  "openspore_in_path": ${openspore_in_path},
  "message": "${message}",
  "status": "${status}"
}
EOF
