# Contributing to Silent Breath MMIO

Thank you for considering contributing to this project! This document outlines the development workflow and CI/CD requirements.

## 🚀 Quick Start

### Prerequisites
- Rust nightly toolchain (pinned via `rust-toolchain.toml`)
- Git
- Familiarity with bare-metal Rust programming

### Setup
```bash
# Clone the repository
git clone https://github.com/LeeMetaX/Silent-Breath-Online.git
cd Silent-Breath-Online

# Install Rust nightly (automatic via rust-toolchain.toml)
rustup show

# Run tests to verify setup
cargo test
```

## 📋 Development Workflow

### 1. Before You Start
- Check existing issues or create a new one
- Discuss major changes before implementation
- Fork the repository and create a feature branch

### 2. Making Changes

#### Branch Naming
```
feature/your-feature-name
bugfix/issue-number-description
test/module-name-tests
docs/documentation-improvement
```

#### Code Standards
- **Formatting**: All code must pass `cargo fmt --check`
- **Linting**: All code must pass `cargo clippy -- -D warnings`
- **Documentation**: Public APIs must have doc comments
- **Testing**: New code requires tests (maintain 100% module coverage)

### 3. Testing Requirements

#### Run All Tests
```bash
# Quick test run
cargo test

# Verbose output
cargo test -- --test-threads=1 --nocapture

# Specific module
cargo test cache_coherency::tests
```

#### Test Coverage Requirement
- **Minimum**: Every public function must have at least one test
- **Target**: 100% module coverage (currently achieved: 11/11 modules)
- **New modules**: Must include comprehensive test suite before PR

### 4. Pre-Commit Checks

Run these commands before committing:
```bash
# Format code
cargo fmt

# Check compilation
cargo check

# Run clippy
cargo clippy --all-targets --all-features -- -D warnings

# Run all tests
cargo test

# Check documentation
cargo doc --no-deps
```

### 5. Commit Message Format

Follow conventional commits:
```
<type>(<scope>): <subject>

<body>

<footer>
```

**Types**: `feat`, `fix`, `test`, `docs`, `refactor`, `perf`, `chore`

**Examples**:
```
feat(mmio): Add support for 16-core configurations

Extends MMIOCoherency to support up to 16 cores with
backward compatibility for existing 8-core setup.

Closes #123

test(ecc): Add Reed-Solomon multi-byte correction tests

Adds comprehensive tests for RS(255,223) encoding with
artificial error injection at multiple byte positions.
```

## 🔍 CI/CD Pipeline

### Automated Checks
Every push and PR triggers:

1. **Test Suite** (175 tests)
   - All tests must pass
   - Debug and release mode validation
   - Multi-platform testing (Linux, macOS, Windows)

2. **Code Quality**
   - Formatting check (`cargo fmt`)
   - Clippy lints (`cargo clippy`)
   - Documentation build

3. **Security Audit**
   - Dependency vulnerability scan (`cargo audit`)

4. **Build Artifacts**
   - Library compilation (debug + release)

### Status Badges
Check the README for current build status. PRs cannot merge with failing checks.

## 📝 Pull Request Process

### Before Submitting
- [ ] All tests pass locally (`cargo test`)
- [ ] Code is formatted (`cargo fmt`)
- [ ] No clippy warnings (`cargo clippy`)
- [ ] Documentation updated (if applicable)
- [ ] CHANGELOG.md updated (for user-facing changes)
- [ ] Tests added for new functionality

### PR Guidelines
1. **Title**: Clear, concise description of changes
2. **Description**:
   - What problem does this solve?
   - How does it solve it?
   - Any breaking changes?
   - Testing performed
3. **Size**: Keep PRs focused and under 50 files when possible
4. **Review**: Address all feedback before requesting re-review

### Review Process
- At least one maintainer approval required
- CI/CD must pass (all checks green)
- No unresolved conversations
- Branch must be up-to-date with main

## 🔧 Module-Specific Guidelines

### MMIO Controllers (`mmio.rs`, `shadow_mmio.rs`)
- All volatile operations must use `read_volatile`/`write_volatile`
- Register layouts must be documented with bit diagrams
- Tests must use mock registers (never real hardware addresses)

### FFI Boundaries (`runtime.rs`, `shadow_runtime.rs`)
- All FFI functions must validate null pointers
- Error codes must be documented
- Memory management must be explicit (Box allocation/deallocation)

### Hardware Integration (`fuse_manager.rs`, `sync_manager.rs`)
- OTP writes must be protected (one-time only)
- CRC verification required for all fuse operations
- Tests must use allocated memory, not arbitrary addresses

### Reliability (`ecc_handler.rs`, `version_control.rs`)
- Error correction must be validated with artificial corruption
- Version history must maintain checksums
- Rollback operations must be atomic

## 🐛 Bug Reports

Include:
- Rust version (`rustc --version`)
- OS and architecture
- Steps to reproduce
- Expected vs actual behavior
- Minimal reproduction code (if applicable)

## 📚 Additional Resources

- [Rust no_std Book](https://docs.rust-embedded.org/book/)
- [MESI Cache Coherency](https://en.wikipedia.org/wiki/MESI_protocol)
- [Memory-Mapped I/O](https://en.wikipedia.org/wiki/Memory-mapped_I/O)
- [Project Documentation](./COMPILATION_REPORT.md)

## 📜 License

By contributing, you agree that your contributions will be licensed under the same license as the project.

## 🤝 Code of Conduct

- Be respectful and constructive
- Focus on technical merit
- Welcome newcomers
- Assume good intent

---

**Questions?** Open an issue or start a discussion!
