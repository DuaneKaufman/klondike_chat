#!/bin/bash

OUTPUT_FILE="rust_merged.txt"
TARGET_DIRECTORY="." # Or specify your directory, e.g., "/path/to/your/files"

# Clear the output file if it exists, or create it
> "$OUTPUT_FILE"

for file in "$TARGET_DIRECTORY"/*.rs; do
    if [ -f "$file" ]; then # Check if it's a regular file
        echo "--- FILE: $(basename "$file") ---" >> "$OUTPUT_FILE"
        cat "$file" >> "$OUTPUT_FILE"
        echo "" >> "$OUTPUT_FILE" # Add a blank line for separation
    fi
done
