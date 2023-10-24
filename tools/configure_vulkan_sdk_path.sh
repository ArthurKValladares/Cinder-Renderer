#!/bin/bash
for entry in ~/VulkanSDK/*/
do
  # We re-export this for every path in the subdir, but that's fine
  export VULKAN_SDK_PATH=$entry
  echo "Setup VULKAN_SDK_PATH: $VULKAN_SDK_PATH"
done