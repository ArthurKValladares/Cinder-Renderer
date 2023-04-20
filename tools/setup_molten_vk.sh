#!/bin/bash
if [ -z ${VULKAN_SDK_PATH+x} ]; then 
    echo "Failed to setup MoltenVK. Please set the variable VULKAN_SDK_PATH to the VulkanSDK path on your machine."
else 
    if [ -d "$VULKAN_SDK_PATH" ]; then
        export VULKAN_SDK=$VULKAN_SDK_PATH/macos
        export DYLD_LIBRARY_PATH=$VULKAN_SDK/lib
        export VK_ICD_FILENAMES=$VULKAN_SDK/share/vulkan/icd.d/MoltenVK_icd.json
        export VK_LAYER_PATH=$VULKAN_SDK/share/vulkan/explicit_layer.d
        echo "Successfully setup MoltenVK."
    else
        echo "Failed to setup MoltenVK. VULKAN_SDK_PATH '$VULKAN_SDK_PATH' does not exist."
    fi
fi