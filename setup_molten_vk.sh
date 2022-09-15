#!/bin/bash
export VULKAN_SDK=$HOME/VulkanSDK/1.3.224.1/macOS
export DYLD_LIBRARY_PATH=$VULKAN_SDK/lib
export VK_ICD_FILENAMES=$VULKAN_SDK/share/vulkan/icd.d/MoltenVK_icd.json
export VK_LAYER_PATH=$VULKAN_SDK/share/vulkan/explicit_layer.d