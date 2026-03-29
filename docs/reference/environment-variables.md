# Environment Variables

## Runtime

| Variable | Description | Default |
|:---------|:------------|:--------|
| `NOTI_CONFIG` | Override config file path | `~/.config/noti/config.toml` |

## Install Script

These variables are used by the install scripts (`install.sh` / `install.ps1`):

| Variable | Description | Default |
|:---------|:------------|:--------|
| `NOTI_INSTALL_DIR` | Custom install directory | `~/.local/bin` (Linux/macOS), `%USERPROFILE%\.noti\bin` (Windows) |
| `NOTI_INSTALL_VERSION` | Pin a specific version | Latest release |
| `NOTI_INSTALL_REPOSITORY` | Override GitHub repository | `loonghao/noti` |

## Examples

```bash
# Use custom config location
NOTI_CONFIG=/etc/noti/config.toml noti send --profile team --message "Hello"

# Install specific version
NOTI_INSTALL_VERSION=v0.1.2 curl -fsSL https://raw.githubusercontent.com/loonghao/noti/main/scripts/install.sh | bash

# Install to custom directory
NOTI_INSTALL_DIR=/opt/bin curl -fsSL https://raw.githubusercontent.com/loonghao/noti/main/scripts/install.sh | bash
```
