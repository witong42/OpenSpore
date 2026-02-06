#!/bin/bash
# tool_validator.sh

# Check for osascript
if command -v osascript &> /dev/null; then
  osascript_status=\"OK\"
else
  osascript_status=\"NOT FOUND\"
fi

# Check for scutil
if command -v scutil &> /dev/null; then
  scutil_status=\"OK\"
else
  scutil_status=\"NOT FOUND\"
fi

# Check for hostnamectl
if command -v hostnamectl &> /dev/null; then
  hostnamectl_status=\"OK\"
else
  hostnamectl_status=\"NOT FOUND\"
fi

echo \"Tool Validation Report:\"
echo \"osascript: ${osascript_status}\"
echo \"scutil: ${scutil_status}\"
echo \"hostnamectl: ${hostnamectl_status}\"