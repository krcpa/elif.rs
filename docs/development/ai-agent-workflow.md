# AI Agent Development Workflow

This document outlines the proven workflow for AI agents working on elif.rs issues, based on successful resolution of GitHub issues.

## Overview

The elif.rs framework is designed to be AI-friendly, with clear patterns and comprehensive documentation. This workflow ensures systematic issue resolution with proper documentation and testing.

## Step-by-Step Workflow

### 1. Issue Investigation
```bash
# Start by understanding the issue
gh issue view <number>

# Create appropriate branch
git checkout -b fix/issue-description-<number>
# or: git checkout -b feature/issue-description-<number>
```

**Key Actions:**
- Read issue description carefully
- Understand expected vs actual behavior
- Identify affected components

### 2. Progress Tracking
Use the `todo_write` tool to manage tasks:
```
- investigate-issue: "Verify the problem by reproducing it"
- trace-root-cause: "Find the exact cause in codebase" 
- design-solution: "Plan and implement the fix"
- test-fix: "Verify the solution works"
- create-pr: "Submit for review"
```

### 3. Investigation Techniques

#### Code Exploration
- Use `codebase_search` for semantic understanding
- Use `grep` for exact text matches
- Use `cargo expand` to understand macro expansions
- Use `read_file` to examine specific implementations

#### Testing and Verification
```bash
# Verify compilation
cargo check

# Run tests
cargo test

# Check macro expansions
cargo expand --bin <target> | grep -A 5 -B 5 "<pattern>"

# Test actual application
cd test-app && cargo run
```

### 4. Root Cause Analysis

**Example from Issue #428:**
1. **Initial hypothesis**: Module import chain broken
2. **Investigation**: Used `cargo expand` to trace controller registration
3. **Discovery**: Controllers with fields incorrectly treated as needing DI
4. **Root cause**: Auto-registration macro skipped for struct fields

**Techniques:**
- Start with broad hypotheses, narrow down systematically
- Use debugging tools (`cargo expand`, logs, etc.)
- Compare working vs non-working examples
- Trace code execution paths

### 5. Solution Implementation

**Guidelines:**
- Make minimal, focused changes
- Maintain backward compatibility
- Follow framework patterns in `.cursorrules`
- Add comprehensive testing
- Document changes thoroughly

### 6. Testing Strategy

**Multi-level verification:**
```bash
# 1. Compilation check
cargo check

# 2. Unit tests
cargo test

# 3. Integration testing
cd test-app && cargo run

# 4. Functionality verification
curl -v http://127.0.0.1:3000/endpoint
```

### 7. Pull Request Creation

**Commit message format:**
```
Fix controller registration issue for submodules

- Remove struct fields from UsersController that interfered with auto-registration
- Controllers with fields were incorrectly treated as needing dependency injection
- Both HealthController and UsersController now register successfully
- Framework's modular architecture now works as intended

Closes #<number>
```

**PR template:**
```markdown
## Problem Solved
Brief description of the issue

## Root Cause
Technical explanation of what was causing the problem

## Solution
Implementation approach and key changes

## Testing
- ✅ Specific test cases
- ✅ Integration verification
- ✅ Regression testing

## Files Changed
- `file1.rs` - Description of changes
- `file2.rs` - Description of changes

Closes #<number>
```

## Issue Discovery Protocol

When discovering new issues during development:

1. **Create new GitHub issue immediately**
```bash
gh issue create --title "Clear, descriptive title" \
  --body "Detailed description with reproduction steps" \
  --label "bug,phase-X"
```

2. **Link in current work**
   - Reference in PR description
   - Update issue descriptions as needed
   - Prioritize based on severity

3. **Document thoroughly**
   - Include reproduction steps
   - Explain impact and scope
   - Provide technical details

## Success Metrics

**A successful issue resolution includes:**
- ✅ Problem completely understood and reproduced
- ✅ Root cause identified with technical explanation
- ✅ Minimal, focused solution implemented
- ✅ Comprehensive testing performed
- ✅ Proper documentation provided
- ✅ Related issues identified and created
- ✅ Clean commit history with proper messages
- ✅ Detailed PR with clear explanation

## Example: Issue #428 Resolution

**Timeline:** Successfully resolved controller registration issue
**Key techniques:** `cargo expand`, semantic codebase search, systematic debugging
**Root cause:** Struct fields interfering with auto-registration macro
**Solution:** Simplified controller to unit struct
**Result:** Both controllers now register successfully

**Related work:** Discovered and created Issue #429 for route registration

This demonstrates the complete workflow from investigation to resolution with proper documentation and follow-up.

## Tips for AI Agents

1. **Use parallel tool calls** - Gather information efficiently
2. **Follow the TodoWrite pattern** - Track progress systematically  
3. **Be thorough in investigation** - Don't skip the analysis phase
4. **Document everything** - Future agents benefit from clear explanations
5. **Test comprehensively** - Verify fix works in multiple scenarios
6. **Create follow-up issues** - Don't ignore discovered problems

## Framework-Specific Notes

- **Controller registration** happens via `ctor` functions when modules are imported
- **Macro expansions** can be debugged with `cargo expand`
- **Auto-registration** is skipped for controllers with dependency injection needs
- **Module system** uses compile-time registry for discovery
- **Bootstrap system** supports zero-boilerplate configuration

Refer to `.cursorrules` and `CLAUDE.md` for framework-specific patterns and guidelines.
