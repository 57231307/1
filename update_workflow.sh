#!/bin/bash

# Update package-release outputs and fetch-depth
sed -i '/name: 创建发布包/a\    outputs:\n      version: ${{ steps.version.outputs.version }}' .github/workflows/ci-cd.yml

# We need to use sed carefully or just rewrite the file.
