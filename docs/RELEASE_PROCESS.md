# elif.rs Release Process

> Standard operating procedure for releasing packages to drive mass adoption

## 🎯 Release Philosophy

**Goal**: Make Rust web development as approachable as Laravel made PHP development

**Strategy**: 
- **Frequent releases** build momentum and confidence
- **Quality over speed** - every release must work perfectly
- **Clear communication** - developers need to understand changes
- **Backward compatibility** - minimize breaking changes

## 📋 Pre-Release Checklist

### **1. Code Quality Validation**
```bash
# Full test suite
cargo test --workspace                   # Must pass: 600+ tests

# Code quality
cargo clippy --workspace -- -D warnings # Zero warnings
cargo fmt --check                       # Proper formatting

# Security audit
cargo audit                             # No vulnerabilities

# Documentation
cargo doc --workspace --no-deps         # All docs build
```

### **2. Version Consistency Check**
```bash
# Verify all inter-package dependencies are updated
grep -r "elif-" crates/*/Cargo.toml | grep version

# Ensure README.md package versions match Cargo.toml
grep -A 20 "Available Packages" README.md
```

### **3. Integration Testing**
```bash
# Test CLI tool with fresh project
elifrs new test-app
cd test-app
cargo build && cargo test

# Test all major workflows
elifrs generate api User
elifrs migrate run
cargo run  # Server should start on :3000
```

## 🚀 Release Execution

### **Step 1: Version Bump Coordination**

**Order of Operations** (dependency order):
1. **Foundation**: `elif-core`, `elif-validation` 
2. **Data Layer**: `elif-orm`, `elif-cache`, `elif-queue`
3. **Security**: `elif-security`, `elif-auth`
4. **Application**: `elif-http`, `elif-storage`, `elif-email`
5. **Development**: `elif-testing`, `elif-openapi*`, `elif-codegen`, `elif-introspect`
6. **CLI**: `elifrs` (depends on all others)

### **Step 2: Automated Release Script**

```bash
#!/bin/bash
# release.sh - Automated elif.rs release

set -e

PACKAGES=(
    "crates/core:elif-core"
    "crates/elif-validation:elif-validation"
    "crates/orm:elif-orm" 
    "crates/elif-cache:elif-cache"
    "crates/elif-queue:elif-queue"
    "crates/elif-security:elif-security"
    "crates/elif-auth:elif-auth"
    "crates/elif-http:elif-http"
    "crates/elif-storage:elif-storage"
    "crates/elif-email:elif-email"
    "crates/elif-testing:elif-testing"
    "crates/elif-openapi-derive:elif-openapi-derive"
    "crates/elif-openapi:elif-openapi"
    "crates/codegen:elif-codegen"
    "crates/introspect:elif-introspect"
    "crates/cli:elifrs"
)

echo "🚀 Starting elif.rs release process..."

# 1. Run full test suite
echo "📋 Running full test suite..."
cargo test --workspace

# 2. Release packages in dependency order
for package_info in "${PACKAGES[@]}"; do
    IFS=':' read -r path name <<< "$package_info"
    echo "📦 Publishing $name..."
    
    cd "$path"
    cargo publish --dry-run  # Verify first
    cargo publish
    cd - > /dev/null
    
    echo "✅ $name published successfully"
    sleep 10  # Rate limit protection
done

echo "🎉 All packages published successfully!"
```

### **Step 3: Post-Release Actions**

1. **Update Documentation**
   ```bash
   # Update README.md with new versions
   # Update docs/FRAMEWORK_ARCHITECTURE.md
   # Update examples with latest versions
   ```

2. **GitHub Release**
   ```bash
   # Create GitHub release with changelog
   gh release create v0.7.0 \
     --title "elif.rs v0.7.0: WebSocket Channels & Performance" \
     --notes-file CHANGELOG.md
   ```

3. **Community Announcement**
   - Reddit r/rust post
   - Twitter/X announcement  
   - HackerNews submission (if major release)
   - Blog post with migration guide

## 📈 Success Tracking

### **Metrics to Monitor**
```bash
# Download stats (weekly)
cargo search elif- | grep Downloads

# GitHub activity
gh repo view krcpa/elif.rs --json stargazers_count,forks_count

# Community engagement
gh issue list --state open
gh pr list --state open
```

### **Release Success Criteria**

**Minor Release (0.X.0)**:
- [ ] All tests pass (600+)
- [ ] Zero clippy warnings
- [ ] Documentation updated
- [ ] At least 2 new features/improvements
- [ ] Backward compatible

**Major Release (X.0.0)**:
- [ ] Migration guide published
- [ ] Breaking changes documented
- [ ] Performance benchmarks
- [ ] Community feedback incorporated
- [ ] Tutorial/blog post ready

## 🔄 Continuous Improvement

### **Weekly Release Rhythm**
- **Monday**: Plan releases, review issues
- **Tuesday-Thursday**: Development, testing
- **Friday**: Release preparation, documentation
- **Weekend**: Community feedback, planning

### **Quality Gates**
1. **Code Review**: All changes reviewed
2. **Test Coverage**: Maintain 600+ passing tests
3. **Performance**: No regressions in benchmarks
4. **Documentation**: Always up-to-date
5. **Security**: Regular audits with `cargo audit`

## 🎉 The Laravel Moment Strategy

### **What We're Building Toward**

**Laravel's Success Formula**:
1. **Incredible DX**: `php artisan make:*` → `elifrs make *` ✅
2. **Everything Included**: No package hunting → 17 packages ✅
3. **Great Docs**: Laravel docs legendary → Framework tree docs ✅
4. **Active Community**: Laracasts, forums → Need to build

**elif.rs is Ready for Mass Adoption**:
- ✅ **Complete Framework**: 17 packages, 600+ tests
- ✅ **Laravel-like DX**: Simple, obvious APIs
- ✅ **Production Features**: Auth, caching, queues, WebSocket
- 🚧 **Community Building**: Tutorials, showcases needed

### **Next 90 Days: The Push**

**Month 1**: Stabilize to 0.7.x versions
**Month 2**: Performance optimization, benchmarks
**Month 3**: Community content, showcase projects

**Target**: Become the **default choice** for Rust web development

---

**🚀 Just like Laravel transformed PHP in 2014, elif.rs can transform Rust web development in 2025**