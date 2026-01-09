# CI/CD Status Badges

Add these badges to the top of your README.md:

```markdown
# Silent Breath MMIO

[![Rust CI/CD](https://github.com/LeeMetaX/Silent-Breath-Online/actions/workflows/rust-ci.yml/badge.svg)](https://github.com/LeeMetaX/Silent-Breath-Online/actions/workflows/rust-ci.yml)
[![PR Checks](https://github.com/LeeMetaX/Silent-Breath-Online/actions/workflows/pr-checks.yml/badge.svg)](https://github.com/LeeMetaX/Silent-Breath-Online/actions/workflows/pr-checks.yml)
![Tests](https://img.shields.io/badge/tests-175%20passing-brightgreen)
![Coverage](https://img.shields.io/badge/coverage-11%2F11%20modules-brightgreen)
![Rust](https://img.shields.io/badge/rust-nightly-orange)
![Platform](https://img.shields.io/badge/platform-linux%20%7C%20macos%20%7C%20windows-blue)
```

## Badge Preview

### CI Status
- ✅ **Rust CI/CD**: Main pipeline status (test, build, quality, security)
- ✅ **PR Checks**: Pull request validation status

### Project Metrics
- 🧪 **Tests**: 175 tests passing
- 📦 **Coverage**: 11/11 modules (100%)
- 🦀 **Rust**: Nightly toolchain
- 🖥️ **Platform**: Linux, macOS, Windows

## Custom Badges

### Test Count
```markdown
![Tests](https://img.shields.io/badge/tests-175%20passing-brightgreen)
```

### Module Coverage
```markdown
![Coverage](https://img.shields.io/badge/coverage-11%2F11%20modules-brightgreen)
```

### Build Status
```markdown
[![Build](https://img.shields.io/badge/build-passing-brightgreen)](https://github.com/LeeMetaX/Silent-Breath-Online/actions)
```

### License (if applicable)
```markdown
![License](https://img.shields.io/badge/license-MIT-blue)
```

## Usage in README

Place badges at the top of your README.md, after the title:

```markdown
# Silent Breath MMIO

> High-performance MMIO cache coherency and shadow register system for bare-metal Rust

[![CI/CD](https://github.com/LeeMetaX/Silent-Breath-Online/actions/workflows/rust-ci.yml/badge.svg)](https://github.com/LeeMetaX/Silent-Breath-Online/actions)
![Tests](https://img.shields.io/badge/tests-175%20passing-brightgreen)
![Coverage](https://img.shields.io/badge/coverage-100%25-brightgreen)

[Description of your project here...]
```
