# GitHub Actions Workflows

## Project Automation

The `project-automation.yml` workflow automatically manages the GitHub Project for elif.rs development.

### Features

1. **Automatic Project Addition**: Issues and PRs labeled with any phase label (`phase-1` through `phase-6`) are automatically added to the project.

2. **Priority Assignment**: 
   - Phase 1-3: High priority (core functionality)
   - Phase 4-5: Medium priority (developer experience and production features)
   - Phase 6: Low priority (advanced features)

3. **Status Management**: New items are automatically set to "Todo" status.

### Setup Requirements

To use this workflow, you need to:

1. Create a GitHub Personal Access Token with `project` scope
2. Add it as a repository secret named `PROJECT_TOKEN`
3. Ensure the project URL is correct in the workflow file

### Usage

Simply create issues or PRs with phase labels:

```bash
# This will automatically add to project with High priority
gh issue create --title "New feature" --label "phase-1,enhancement"

# This will automatically add to project with Medium priority  
gh issue create --title "CLI improvement" --label "phase-4,enhancement"
```

The automation handles the rest!