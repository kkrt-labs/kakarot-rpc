#!/bin/bash
set -o pipefail

# Directory containing the original JSON files
src_dir="./lib/kakarot/build"

# Root build directory
root_dir="./.kakarot"
# Directory to write the new JSON files to
dest_dir="/artifacts"
# Directory to write the entire build to
build_dir="/build"

# Ensure the destination directory exists
rm -rf "${root_dir}"
mkdir -p "${root_dir}"

# Find and process each JSON file
find "${src_dir}" -type f -name '*.json' | while read -r src_file; do
	# Extract the filename without the extension
	filename=$(basename -- "${src_file}")
	filename="${filename%.*}"

	# Check and create a subdirectory structure in destination if needed
	sub_dir=$(dirname "${src_file}")
	sub_dir=${sub_dir#"${src_dir}"}
	mkdir -p "${root_dir}${dest_dir}${sub_dir}"
	mkdir -p "${root_dir}${build_dir}${sub_dir}"

	# Use jq to extract the 'abi' field and write it to a new JSON file
	jq '.abi' "${src_file}" >"${root_dir}${dest_dir}${sub_dir}/${filename}.json"

	# Copy the original JSON file to the build directory
	cp "${src_file}" "${root_dir}${build_dir}${sub_dir}/${filename}.json"
done
