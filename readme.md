# Himawari HSD Data Downloader

一个用于下载Himawari卫星HSD（Himawari Standard Data）数据的多线程Rust工具。该工具支持高效的批量下载、断点续传、数据完整性检查和智能文件组织。

## 目录

- [特性](#特性)
- [安装](#安装)
- [快速开始](#快速开始)
- [详细配置](#详细配置)
- [使用指南](#使用指南)
- [高级功能](#高级功能)
- [故障排除](#故障排除)
- [开发](#开发)

## 特性

### 核心功能
- **多线程并行下载**：支持可配置的线程数量，大幅提升下载效率
- **断点续传**：自动检测并恢复未完成的下载
- **智能文件组织**：按时间层次结构组织文件（年/月/日/时）
- **数据完整性检查**：自动验证文件大小和数据完整性
- **波段筛选**：支持下载特定波段数据（如可见光波段B01-B03）
- **流式下载**：边下载边写入磁盘，节省内存使用

### 技术特性
- **SFTP协议**：使用SSH2协议安全传输数据
- **配置管理**：支持TOML格式配置文件和交互式配置
- **错误处理**：完善的错误重试机制和日志记录
- **跨平台**：支持Windows、macOS和Linux系统

## 安装

### 系统要求
- Rust 1.88.0 或更高版本
- 稳定的网络连接
- 足够的磁盘空间（HSD文件通常较大）

### 从源码编译

```bash
# 克隆仓库
git clone https://github.com/your-username/Himawari_HSD_downloader.git
cd Himawari_HSD_downloader

# 编译项目
cargo build --release

# 运行程序
./target/release/Himawari_HSD_downloader
```
### 依赖项
项目使用以下主要依赖：
- `ssh2` (0.9.5) - SSH2协议支持
- `chrono` (0.4.41) - 日期时间处理
- `toml` (0.9.2) - 配置文件解析
- `serde` (1.0.219) - 序列化/反序列化

## 快速开始
### 1. 首次运行
``` bash
# 运行程序（首次运行会创建默认配置文件）
./target/release/Himawari_HSD_downloader
```
### 2. 配置服务器信息
程序会提示您输入服务器信息,
这里需要先去``` www.eorc.jaxa.jp/ptree/index.html``` 根据User Guide注册账号
``` 
=== 服务器配置 ===
请输入服务器地址 (例如: server.com): ftp.ptree.jaxa.jp
请输入端口号 [22]: 2051
请输入用户名: your_username
请输入密码: your_password

=== 下载配置 ===
请输入线程数 [4]: 4
请输入下载目录 [./himawari_data]: ./data
```
### 3. 选择下载时间
程序会提示您输入下载时间范围：
``` 
Current UTC Time: 2025-07-17 12:00:00
Input download start time(UTC Time): (2025-07-17 10:00:00)
2025-07-17 10:00:00

Input download end time(UTC Time): (2025-07-17 10:00:00)(Use start time instead if input nothing.)
2025-07-17 11:00:00
```
### 4. 开始下载
程序会自动开始下载并显示进度：
``` 
开始下载可见光波段数据...
找到 18 个文件需要下载
线程 0 开始处理 6 个文件
线程 1 开始处理 6 个文件
线程 2 开始处理 6 个文件
...
下载完成！
```
## 详细配置
### 配置文件格式
配置文件 采用TOML格式： `config.toml`
``` toml
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
### 配置选项说明
#### 服务器配置 (`[server]`)
- `host`: 服务器地址（必填）
- : 用户名（必填） `username`
- : 密码（必填） `password`
- `port`: SSH端口号（默认：22）

#### 下载配置 (`[download]`)
- `num_threads`: 下载线程数（默认：4）
- `base_path`: 数据下载目录（默认：`./himawari_data`）
- `organize_by_time`: 是否按时间组织文件（默认：true）
- : 是否保持原始目录结构（默认：false） `keep_original_structure`

## 使用指南
### 时间格式说明
程序使用UTC时间，格式为：`YYYY-MM-DD HH:MM:SS`
例如：
- `2025-07-17 10:00:00` - 2025年7月17日10时00分00秒（UTC）
- `2025-07-17 10:10:00` - 2025年7月17日10时10分00秒（UTC）

### 文件组织方式
当 `organize_by_time = true` 时，文件会按以下结构组织：
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
### 波段说明
程序默认下载可见光波段（B01-B03）：
- **B01**: 0.47 μm（蓝色）
- **B02**: 0.51 μm（绿色）
- **B03**: 0.64 μm（红色）

### 文件命名规则
HSD文件遵循以下命名规则：
``` 
HS_H09_YYYYMMDD_HHMM_BXX_FLDK_R05_S0101.DAT.bz2
```
其中：
- `HS_H09`: Himawari-9卫星标识
- `YYYYMMDD`: 年月日
- `HHMM`: 时分
- `BXX`: 波段编号（如B01, B02, B03等）
- `FLDK`: 全圆盘数据
- `R05`: 分辨率标识
- `S0101`: 段标识

## 高级功能
### 1. 自定义波段下载
虽然程序默认下载可见光波段，但代码支持扩展到其他波段：
``` rust
// 在代码中可以修改波段列表
let custom_bands = vec!["B04".to_string(), "B05".to_string(), "B06".to_string()];
```
### 2. 断点续传
程序自动支持断点续传：
- 检测未完成的下载文件（`.downloading`后缀）
- 自动从上次中断的位置继续下载
- 验证文件完整性

### 3. 数据完整性检查
程序提供完整性检查功能：
- 文件大小验证
- 波段数据完整性报告
- 缺失文件检测

### 4. 性能优化
- **缓冲I/O**: 使用32KB缓冲区优化读写性能
- **内存管理**: 流式下载避免大文件占用过多内存
- **并发控制**: 智能线程调度避免资源争用

## 故障排除
### 常见问题
#### 1. 连接失败
``` 
错误: 线程 0 连接失败: Connection refused
```
**解决方案**：
- 检查服务器地址和端口是否正确
- 确认网络连接正常
- 验证防火墙设置

#### 2. 认证失败
``` 
错误: 线程 0 认证失败: Authentication failed
```
**解决方案**：
- 检查用户名和密码是否正确
- 确认账户未被锁定
- 验证SSH密钥配置

#### 3. 权限问题
``` 
错误: 无法创建目录: Permission denied
```
**解决方案**：
- 确保对下载目录有写入权限
- 检查磁盘空间是否充足
- 考虑使用sudo权限（谨慎使用）

#### 4. 时间格式错误
``` 
错误: Input time is greater than current time
```
**解决方案**：
- 使用UTC时间而非本地时间
- 确保时间格式正确：`YYYY-MM-DD HH:MM:SS`
- 检查开始时间是否早于结束时间

### 日志分析
程序会输出详细的日志信息：
``` 
=== 下载统计摘要 ===
总文件数: 18
成功下载: 15
跳过文件: 2
失败文件: 1
总下载量: 1024 MB
耗时: 5m 30s
平均速度: 3.11 MB/s
```
### 性能调优
1. **调整线程数**：
    - 增加线程数可提高下载速度
    - 但过多线程可能导致服务器连接限制
    - 建议根据网络带宽调整（通常4-8个线程）

2. **优化网络设置**：
    - 确保稳定的网络连接
    - 考虑使用有线连接而非WiFi
    - 避免在网络高峰期下载

3. **磁盘I/O优化**：
    - 使用SSD硬盘可提高写入速度
    - 确保足够的磁盘空间
    - 定期清理临时文件

## 开发
### 项目结构
``` 
src/
├── main.rs                     # 程序入口
├── lib.rs                      # 库文件
├── config.rs                   # 配置管理
├── get_download_time_list.rs   # 时间列表生成
└── download_files_from_list.rs # 文件下载功能
```
### 编译选项
``` bash
# 开发模式编译
cargo build

# 发布模式编译（推荐）
cargo build --release
```
### 贡献指南
1. Fork 项目
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 创建Pull Request

### 代码风格
项目遵循Rust官方代码风格指南：
``` bash
# 格式化代码
cargo fmt

# 检查代码
cargo clippy

# 运行所有检查
cargo fmt && cargo clippy && cargo test
```
### 相关信息
**注意**: 请确保您有合法的权限访问Himawari HSD数据服务器，并遵守相关的数据使用条款。


