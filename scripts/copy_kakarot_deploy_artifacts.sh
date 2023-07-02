#!/bin/bash

# Load environment variables from the .env file
source .env

# Check if the KAKAROT_EVM_PROJECT_DIR variable is set
if [[ -z "${KAKAROT_EVM_PROJECT_DIR}" ]]; then
  echo "Error: KAKAROT_EVM_PROJECT_DIR is not set"
  exit 1
fi

# Check if the COMPILED_KAKAROT_PATH variable is set
if [[ -z "${COMPILED_KAKAROT_PATH}" ]]; then
  echo "Error: COMPILED_KAKAROT_PATH is not set"
  exit 1
fi

# Move to the project directory
cd "${KAKAROT_EVM_PROJECT_DIR}"

# Call make build
make build

# Check if the build was successful
if [[ $? -ne 0 ]]; then
  echo "Error: make build failed"
  cd -  # Return to the previous directory
  exit 1
fi

cd -

# Copy the build files to the destination directory
cp -r "${KAKAROT_EVM_PROJECT_DIR}"/build/* "${COMPILED_KAKAROT_PATH}"

# Check if the copy operation was successful
if [[ $? -ne 0 ]]; then
  echo "Error: Failed to copy the build files"
  cd -  # Return to the previous directory
  exit 1
fi

cd -  # Return to the previous directory

echo "Build files successfully copied to ${COMPILED_KAKAROT_PATH}"
