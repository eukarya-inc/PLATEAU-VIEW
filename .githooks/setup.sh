#!/bin/bash
# Setup script to configure Git hooks for this repository

# Set the hooks directory to .githooks
git config core.hooksPath .githooks

echo "✓ Git hooks configured successfully"
echo "  Hooks directory: .githooks"
echo ""
echo "The following hooks are now active:"
ls -1 .githooks/ | grep -v setup.sh
