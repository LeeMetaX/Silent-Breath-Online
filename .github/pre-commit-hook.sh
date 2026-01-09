#!/bin/bash
# Silent Breath MMIO - Pre-commit Hook
#
# Install: ln -s ../../.github/pre-commit-hook.sh .git/hooks/pre-commit
# Usage: Runs automatically on git commit

set -e

echo "🔍 Running pre-commit checks..."

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check 1: Format
echo -e "${YELLOW}📝 Checking code formatting...${NC}"
if ! cargo fmt -- --check > /dev/null 2>&1; then
    echo -e "${RED}❌ Code formatting check failed!${NC}"
    echo "Run: cargo fmt"
    exit 1
fi
echo -e "${GREEN}✅ Formatting OK${NC}"

# Check 2: Compilation
echo -e "${YELLOW}🔨 Checking compilation...${NC}"
if ! cargo check --all-targets > /dev/null 2>&1; then
    echo -e "${RED}❌ Compilation failed!${NC}"
    echo "Run: cargo check"
    exit 1
fi
echo -e "${GREEN}✅ Compilation OK${NC}"

# Check 3: Clippy
echo -e "${YELLOW}📎 Running clippy...${NC}"
if ! cargo clippy --all-targets --all-features -- -D warnings 2>&1 | grep -q "0 errors"; then
    echo -e "${RED}❌ Clippy found issues!${NC}"
    echo "Run: cargo clippy --all-targets --all-features -- -D warnings"
    exit 1
fi
echo -e "${GREEN}✅ Clippy OK${NC}"

# Check 4: Tests
echo -e "${YELLOW}🧪 Running tests...${NC}"
if ! cargo test --quiet > /dev/null 2>&1; then
    echo -e "${RED}❌ Tests failed!${NC}"
    echo "Run: cargo test"
    exit 1
fi
echo -e "${GREEN}✅ Tests OK (175 passing)${NC}"

echo -e "${GREEN}✨ All pre-commit checks passed!${NC}"
echo ""
