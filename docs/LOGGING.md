# Logging and Troubleshooting

## Log Locations

Application logs are stored in platform-specific locations:

- **Linux**: `~/.local/share/efm/logs/`
- **macOS**: `~/Library/Application Support/efm/logs/`
- **Windows**: `C:\Users\<User>\AppData\Local\efm\logs\`

Logs are rotated daily with the naming pattern `app.log.YYYY-MM-DD`.

## Log Format

- **Console Output**: Human-readable format for development and debugging
- **File Output**: JSON format for structured logging and bug reports

## Log Levels

The application logs at different levels:
- **error**: Critical errors that prevent operation
- **warn**: Warnings that don't stop operation but indicate problems
- **info**: General informational messages about application flow
- **debug**: Detailed information for troubleshooting (enabled for service layer)
- **trace**: Very verbose output (disabled by default)

## Changing Log Level

### For Development

Set the `RUST_LOG` environment variable before running:

```bash
# Show all debug messages
RUST_LOG=debug efm-relm4-ui

# Show trace for specific module
RUST_LOG=service=trace efm-relm4-ui

# Mix levels for different modules
RUST_LOG=service=trace,database=debug,info efm-relm4-ui
```

### For Production

The default log level is `info` with `debug` enabled for the service layer. This provides good troubleshooting information without excessive verbosity.

## Reporting Bugs

When reporting bugs, please include:

1. **Log files** from `~/.local/share/efm/logs/`
2. **Steps to reproduce** the issue
3. **Expected behavior** vs actual behavior
4. **System information** (OS, version, etc.)

### Finding Recent Errors

Look for recent log files with errors:

```bash
# On Linux/macOS
cd ~/.local/share/efm/logs/
grep -r "\"level\":\"ERROR\"" *.log

# View today's log
cat app.log.$(date +%Y-%m-%d) | jq 'select(.level=="ERROR")'
```

## Example Log Entries

### Successful Operation
```json
{
  "timestamp": "2025-11-05T21:15:00.123Z",
  "level": "INFO",
  "message": "Download completed",
  "target": "service::file_set_download::service",
  "successful": 5,
  "failed": 0,
  "span": {
    "name": "download_file_set",
    "file_set_id": 123
  }
}
```

### Error
```json
{
  "timestamp": "2025-11-05T21:15:05.456Z",
  "level": "ERROR",
  "message": "Failed to fetch file set",
  "target": "service::file_set_download::steps",
  "error": "Database connection lost",
  "file_set_id": 123
}
```

## Privacy Note

Log files may contain:
- File names and paths
- Database identifiers
- Cloud storage keys
- Error messages

Please review logs before sharing publicly and redact any sensitive information.
