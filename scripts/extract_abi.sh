#!/bin/bash
set -o pipefail

# Directory containing the original JSON files
src_dir="./lib/kakarot/build"

# Directory to write the new JSON files to
dest_dir="./artifacts"

# Ensure the destination directory exists
rm -rf "${dest_dir}"
mkdir -p "${dest_dir}"

# Find and process each JSON file
find "${src_dir}" -type f -name '*.json' | while read -r src_file; do
	# Extract the filename without the extension
	filename=$(basename -- "${src_file}")
	filename="${filename%.*}"

	# Check and create a subdirectory structure in destination if needed
	sub_dir=$(dirname "${src_file}")
	sub_dir=${sub_dir#"${src_dir}"}
	mkdir -p "${dest_dir}${sub_dir}"

	# Use jq to extract the 'abi' field and write it to a new JSON file
	jq '.abi' "${src_file}" >"${dest_dir}${sub_dir}/${filename}.json"
done
