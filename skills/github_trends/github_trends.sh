#!/bin/bash

CACHE_FILE="/tmp/github_trends_cache.json"
CACHE_EXPIRY=900 # 15 minutes in seconds

# Check if cache file exists and is not expired
if [ -f "$CACHE_FILE" ] && [ $(( $(date +%s) - $(stat -f %m "$CACHE_FILE") )) -lt "$CACHE_EXPIRY" ]; then
    cat "$CACHE_FILE"
    exit 0
fi

# GitHub API endpoint and query parameters
API_URL="https://api.github.com/search/repositories"
QUERY="q=stars:>1&sort=stars&order=desc" # Find repos with more than 1 star, sorted by stars

# Fetch data from GitHub API using curl and jq
DATA=$(curl -s "$API_URL?$QUERY")

# Check if curl command was successful
if [ $? -ne 0 ]; then
    echo "{\"success\": false, \"message\": \"Failed to fetch data from GitHub API\"}"
    exit 1
fi

# Extract relevant information using jq
TRENDING_REPOS=$(echo "$DATA" | jq -c '.items[] | {name: .name, description: .description, stars: .stargazers_count}')

# Enclose the output in square brackets to form a valid JSON array
JSON_OUTPUT="["
i=0
while IFS= read -r line; do
    if [ $i -gt 0 ]; then
        JSON_OUTPUT="$JSON_OUTPUT,"
    fi
    JSON_OUTPUT="$JSON_OUTPUT$line"
    i=$((i+1))
done <<< "$TRENDING_REPOS"
JSON_OUTPUT="$JSON_OUTPUT]"

# Check if jq command was successful
if [ $? -ne 0 ]; then
    echo "{\"success\": false, \"message\": \"Failed to parse JSON data\"}"
    exit 1
fi

# Output the JSON data
echo "$JSON_OUTPUT"

# Save the output to the cache file
echo "$JSON_OUTPUT" > "$CACHE_FILE"
