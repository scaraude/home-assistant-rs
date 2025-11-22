#!/bin/bash
# Install Git hooks for this project

set -e

echo "Installing Git hooks..."

# Copy pre-commit hook
cp hooks/pre-commit .git/hooks/pre-commit
chmod +x .git/hooks/pre-commit

# Copy pre-push hook
cp hooks/pre-push .git/hooks/pre-push
chmod +x .git/hooks/pre-push

echo "✅ Git hooks installed successfully!"
echo ""
echo "📋 Installed hooks:"
echo "  • pre-commit: Automatically builds and commits ARM64 binary when Rust files change"
echo "  • pre-push: Validates ARM64 binary is built before pushing"
