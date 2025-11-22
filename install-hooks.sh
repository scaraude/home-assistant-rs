#!/bin/bash
# Install Git hooks for this project

set -e

echo "Installing Git hooks..."

# Copy pre-push hook
cp hooks/pre-push .git/hooks/pre-push
chmod +x .git/hooks/pre-push

echo "✅ Git hooks installed successfully!"
echo ""
echo "The pre-push hook will automatically build the ARM64 binary before each push."
