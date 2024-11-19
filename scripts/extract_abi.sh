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

# Start logging
echo "Starting build process"

# Ensure the destination directory exists
echo "Removing existing build directory ${root_dir}"
rm -rf "${root_dir}"
echo "Creating new build directory ${root_dir}"
mkdir -p "${root_dir}"

# Start processing JSON files
echo "Finding JSON files in ${src_dir} to process..."
find "${src_dir}" -type f -name '*.json' | while read -r src_file; do
	# Extract the filename without the extension
	filename=$(basename -- "${src_file}")
	filename="${filename%.*}"
	echo "Processing file: ${src_file}"

	# Check and create a subdirectory structure in destination if needed
	sub_dir=$(dirname "${src_file}")
	sub_dir=${sub_dir#"${src_dir}"}
	echo "Creating subdirectories in ${root_dir}${dest_dir}/${sub_dir} and ${root_dir}${build_dir}/${sub_dir}"
	mkdir -p "${root_dir}${dest_dir}${sub_dir}"
	mkdir -p "${root_dir}${build_dir}${sub_dir}"

	# Use jq to extract the 'abi' field and write it to a new JSON file
	jq '.abi' "${src_file}" >"${root_dir}${dest_dir}${sub_dir}/${filename}.json"

	# Copy the original JSON file to the build directory
	cp "${src_file}" "${root_dir}${build_dir}${sub_dir}/${filename}.json"
done

echo "Build process complete."
