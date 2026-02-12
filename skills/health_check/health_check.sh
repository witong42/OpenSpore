#!/bin/bash

# Check if the engine directory exists
ENGINE_DIR="$HOME/.openspore"
if [ -d "$ENGINE_DIR" ]; then
    engine_dir_exists=true
    engine_dir_message="Engine directory exists at $ENGINE_DIR."
else
    engine_dir_exists=false
    engine_dir_message="Engine directory does NOT exist at $ENGINE_DIR!"
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
if [ "$engine_dir_exists" = "true" ] && [ "$openspore_in_path" = "true" ]; then
    status="OK"
    message="All health checks passed."
else
    status="ERROR"
    message="One or more health checks failed! Engine: $engine_dir_message, OpenSpore: $openspore_path_message"
fi

# Output JSON
cat <<EOF
{
  "engine_dir_exists": ${engine_dir_exists},
  "openspore_in_path": ${openspore_in_path},
  "message": "${message}",
  "status": "${status}"
}
EOF
