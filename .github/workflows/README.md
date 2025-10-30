# GitHub Workflows

This directory contains GitHub Actions workflows for the project.

## Workflows

### rust.yml - Continuous Integration
Runs on every push to main and on pull requests. It:
- Installs dependencies (GTK4, DBus, etc.)
- Runs database migrations
- Builds the project
- Runs all tests

### deb-package.yml - Debian Package Build
Creates a Debian package for the application. Can be triggered:
- **Manually**: Go to Actions > Build Debian Package > Run workflow
- **On tags**: Push a tag starting with `v` (e.g., `v0.1.0`)

#### Manual Trigger
```bash
# Via GitHub UI: Actions > Build Debian Package > Run workflow

# Or via gh CLI:
gh workflow run deb-package.yml
```

#### Tag-based Release
```bash
git tag v0.1.0
git push origin v0.1.0
```

This will:
1. Build a release binary
2. Create a `.deb` package
3. Upload it as an artifact
4. Create a GitHub release with the package attached (if triggered by a tag)

#### Installing the Package
Download the `.deb` file from the workflow artifacts or GitHub releases and install:
```bash
sudo dpkg -i efm-relm4-ui_0.1.0-1_amd64.deb
sudo apt-get install -f  # Install any missing dependencies
```
