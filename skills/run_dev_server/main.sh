#!/bin/bash

directory=$1
logfile=/tmp/dev_server.log

cd "$directory" || exit 1

bun run dev > "$logfile" 2>&1 &

echo "Development server started. Outputting logs to $logfile"
