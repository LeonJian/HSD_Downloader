# Himawari HSD Data Downloader

A multi-threaded Rust tool for downloading Himawari satellite HSD (Himawari Standard Data) data. This tool supports efficient batch downloading, resume capability, data integrity checking, and intelligent file organization.

## Table of Contents

- [Features](#features)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [Detailed Configuration](#detailed-configuration)
- [Usage Guide](#usage-guide)
- [Advanced Features](#advanced-features)
- [Troubleshooting](#troubleshooting)
- [Development](#development)

## Features

### Core Functionality
- **Multi-threaded Parallel Downloads**: Supports configurable thread count for significantly improved download efficiency
- **Intelligent File Organization**: Organizes files in a time-based hierarchy (year/month/day/hour)
- **Data Integrity Checking**: Automatically verifies file size and data integrity
- **Band Filtering**: Supports downloading specific band data (such as visible light bands B01-B03)
- **Streaming Downloads**: Downloads and writes to disk simultaneously to conserve memory usage
- **Resume Capability**: The program automatically detects interrupted downloads and continues from the breakpoint, eliminating the need to restart downloads and saving time and bandwidth.
### Technical Features
- **SFTP Protocol**: Uses SSH2 protocol for secure data transfer
- **Configuration Management**: Supports TOML format configuration files and interactive configuration
- **Error Handling**: Comprehensive error retry mechanism and logging
- **Cross-platform**: Supports Windows, macOS, and Linux systems

## Installation

### System Requirements
- Rust 1.88.0 or higher
- Stable network connection
- Sufficient disk space (HSD files are typically large)

### Compiling from Source

```bash
# Clone the repository
git clone https://github.com/your-username/Himawari_HSD_downloader.git
cd Himawari_HSD_downloader

# Compile the project
cargo build --release

# Run the program
./target/release/Himawari_HSD_downloader
```

### Dependencies
The project uses the following main dependencies:
- `ssh2` (0.9.5) - SSH2 protocol support
- `chrono` (0.4.41) - Date and time handling
- `toml` (0.9.2) - Configuration file parsing
- `serde` (1.0.219) - Serialization/deserialization

## Quick Start

### 1. First Run
```bash
# Run the program (first run will create a default configuration file)
./target/release/Himawari_HSD_downloader
```

### 2. Configure Server Information
The program will prompt you to enter server information.
You need to first register an account at `www.eorc.jaxa.jp/ptree/index.html` according to the User Guide.
```
=== Server Configuration ===
Please enter server address (e.g., server.com): ftp.ptree.jaxa.jp
Please enter port number [22]: 2051
Please enter username: your_username
Please enter password: your_password

=== Download Configuration ===
Please enter thread count [4]: 4
Please enter download directory [./himawari_data]: ./data
```

### 3. Select Download Time
The program will prompt you to enter a download time range:
```
Current UTC Time: 2025-07-17 12:00:00
Input download start time(UTC Time): (2025-07-17 10:00:00)
2025-07-17 10:00:00

Input download end time(UTC Time): (2025-07-17 10:00:00)(Use start time instead if input nothing.)
2025-07-17 11:00:00
```

### 4. Start Download
The program will automatically start downloading and display progress:
```
Starting to download visible light band data...
Found 18 files to download
Thread 0 starting to process 6 files
Thread 1 starting to process 6 files
Thread 2 starting to process 6 files
...
Download complete!
```

## Detailed Configuration

### Configuration File Format
The configuration file uses TOML format: `config.toml`
```toml
[server]
host = "your-server.com"
username = "your_username"
password = "your_password"
port = 22

[download]
num_threads = 4
base_path = "./himawari_data"
organize_by_time = true
keep_original_structure = false
```

### Configuration Options

#### Server Configuration (`[server]`)
- `host`: Server address (required)
- `username`: Username (required)
- `password`: Password (required)
- `port`: SSH port number (default: 22)

#### Download Configuration (`[download]`)
- `num_threads`: Download thread count (default: 4)
- `base_path`: Data download directory (default: `./himawari_data`)
- `organize_by_time`: Whether to organize files by time (default: true)
- `keep_original_structure`: Whether to maintain the original directory structure (default: false)

## Usage Guide

### Time Format Description
The program uses UTC time in the format: `YYYY-MM-DD HH:MM:SS`
Examples:
- `2025-07-17 10:00:00` - July 17, 2025, 10:00:00 (UTC)
- `2025-07-17 10:10:00` - July 17, 2025, 10:10:00 (UTC)

### File Organization Method
When `organize_by_time = true`, files are organized in the following structure:
```
himawari_data/
├── 2025/
│   └── 07/
│       └── 17/
│           └── 10/
│               ├── HS_H09_20250717_1000_B01_FLDK_R05_S0101.DAT.bz2
│               ├── HS_H09_20250717_1000_B02_FLDK_R05_S0101.DAT.bz2
│               └── HS_H09_20250717_1000_B03_FLDK_R05_S0101.DAT.bz2
```

### Band Description
The program downloads visible light bands (B01-B03) by default:
- **B01**: 0.47 μm (Blue)
- **B02**: 0.51 μm (Green)
- **B03**: 0.64 μm (Red)

### File Naming Convention
HSD files follow this naming convention:
```
HS_H09_YYYYMMDD_HHMM_BXX_FLDK_R05_S0101.DAT.bz2
```
Where:
- `HS_H09`: Himawari-9 satellite identifier
- `YYYYMMDD`: Year, month, day
- `HHMM`: Hour, minute
- `BXX`: Band number (e.g., B01, B02, B03, etc.)
- `FLDK`: Full disk data
- `R05`: Resolution identifier
- `S0101`: Segment identifier

## Advanced Features

### 1. Custom Band Download
While the program downloads visible light bands by default, the code supports extension to other bands:
``` rust
// In the code, you can modify the band list
let custom_bands = vec!["B04".to_string(), "B05".to_string(), "B06".to_string()];
```

### 2. Resume Capability
The program automatically supports resuming downloads:
- Detects incomplete download files (with `.downloading` suffix)
- Automatically continues downloading from the last interrupted position
- Verifies file integrity
- Supports resuming after network interruptions or program crashes
- Displays resume progress and completion percentage
- Intelligently determines breakpoint position based on file header information

### 3. Data Integrity Check
The program provides integrity checking functionality:
- File size verification
- Band data integrity reporting
- Missing file detection

### 4. Performance Optimization
- **Buffered I/O**: Uses 32KB buffers to optimize read/write performance
- **Memory Management**: Streaming downloads avoid excessive memory usage for large files
- **Concurrency Control**: Intelligent thread scheduling to avoid resource contention

## Troubleshooting

### Common Issues

#### 1. Connection Failure
```
Error: Thread 0 connection failed: Connection refused
```
**Solution**:
- Check if the server address and port are correct
- Confirm the network connection is normal
- Verify firewall settings

#### 2. Authentication Failure
```
Error: Thread 0 authentication failed: Authentication failed
```
**Solution**:
- Check if the username and password are correct
- Confirm the account is not locked
- Verify SSH key configuration

#### 3. Permission Issues
```
Error: Cannot create directory: Permission denied
```
**Solution**:
- Ensure you have write permissions for the download directory
- Check if there is sufficient disk space
- Consider using sudo permissions (use with caution)

#### 4. Time Format Error
```
Error: Input time is greater than current time
```
**Solution**:
- Use UTC time instead of local time
- Ensure the time format is correct: `YYYY-MM-DD HH:MM:SS`
- Check that the start time is earlier than the end time

### Log Analysis
The program outputs detailed log information:
```
=== Download Statistics Summary ===
Total files: 18
Successfully downloaded: 15
Skipped files: 2
Failed files: 1
Total download size: 1024 MB
Time elapsed: 5m 30s
Average speed: 3.11 MB/s
```

### Performance Tuning

1. **Adjust Thread Count**:
   - Increasing thread count can improve download speed
   - However, too many threads may lead to server connection limits
   - Recommended to adjust based on network bandwidth (typically 4-8 threads)

2. **Optimize Network Settings**:
   - Ensure a stable network connection
   - Consider using a wired connection instead of WiFi
   - Avoid downloading during network peak hours

3. **Disk I/O Optimization**:
   - Using an SSD can improve write speed
   - Ensure sufficient disk space
   - Regularly clean temporary files

## Development

### Project Structure
```
src/
├── main.rs                     # Program entry
├── lib.rs                      # Library file
├── config.rs                   # Configuration management
├── get_download_time_list.rs   # Time list generation
└── download_files_from_list.rs # File download functionality
```

### Compilation Options
```bash
# Development mode compilation
cargo build

# Release mode compilation (recommended)
cargo build --release
```

### Contribution Guidelines
1. Fork the project
2. Create a feature branch (`git checkout -b feature/AmazingFeature`)
3. Commit your changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the branch (`git push origin feature/AmazingFeature`)
5. Create a Pull Request

### Additional Information
**Note**: Please ensure you have legal permission to access the Himawari HSD data server and comply with the relevant data usage terms.