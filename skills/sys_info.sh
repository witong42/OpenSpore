#!/bin/sh
# Exhaustive system information

# Collect data
OS_INFO="$(uname -a)"
CPU_INFO="$(sysctl -n machdep.cpu.brand_string)"
MEMORY_INFO="$(sysctl -n hw.memsize)"

# Ioreg info (limited to model)


# Disk info (using standard df)
DF_INFO="$(df -h | grep '^/dev/' | awk '{print $1 " " $2 " " $3 " " $4 " " $5 " " $8}')"

# Output JSON
echo "OS Info: $OS_INFO"
echo "CPU Info: $CPU_INFO"
echo "Memory Info: $MEMORY_INFO"
echo "Disk Space Info:\n$DF_INFO"
