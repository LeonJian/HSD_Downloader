use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub username: String,
    pub password: String,
    pub port: u16,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DownloadConfig {
    pub num_threads: usize,
    pub base_path: String,
    pub organize_by_time: bool,
    pub keep_original_structure: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub download: DownloadConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "your_server.com".to_string(),
                username: "your_username".to_string(),
                password: "your_password".to_string(),
                port: 22,
            },
            download: DownloadConfig {
                num_threads: 4,
                base_path: "./himawari_data".to_string(),
                organize_by_time: true,
                keep_original_structure: false,
            },
        }
    }
}

impl Config {
    /// 从配置文件加载配置
    pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let config_content = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&config_content)?;
        Ok(config)
    }

    /// 创建默认配置文件
    pub fn create_default_config(path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let default_config = Config::default();
        let config_content = toml::to_string_pretty(&default_config)?;
        fs::write(path, config_content)?;
        Ok(())
    }

    /// 加载配置，如果文件不存在则创建默认配置
    pub fn load_or_create(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        if Path::new(path).exists() {
            Self::from_file(path)
        } else {
            println!("配置文件不存在，正在创建默认配置文件: {}", path);
            Self::create_default_config(path)?;
            println!("请编辑配置文件后重新运行程序");
            Err("需要配置服务器信息".into())
        }
    }

    /// 交互式配置服务器信息
    pub fn interactive_setup() -> Result<Self, Box<dyn std::error::Error>> {
        println!("=== 服务器配置 ===");

        print!("请输入服务器地址 (例如: server.com): ");
        io::stdout().flush()?;
        let mut host = String::new();
        io::stdin().read_line(&mut host)?;
        let host = host.trim().to_string();

        print!("请输入端口号 [22]: ");
        io::stdout().flush()?;
        let mut port_input = String::new();
        io::stdin().read_line(&mut port_input)?;
        let port = if port_input.trim().is_empty() {
            22
        } else {
            port_input
                .trim()
                .parse::<u16>()
                .map_err(|_| "无效的端口号")?
        };

        print!("请输入用户名: ");
        io::stdout().flush()?;
        let mut username = String::new();
        io::stdin().read_line(&mut username)?;
        let username = username.trim().to_string();

        print!("请输入密码: ");
        io::stdout().flush()?;
        let mut password = String::new();
        io::stdin().read_line(&mut password)?;
        let password = password.trim().to_string();

        println!("\n=== 下载配置 ===");

        print!("请输入线程数 [4]: ");
        io::stdout().flush()?;
        let mut threads_input = String::new();
        io::stdin().read_line(&mut threads_input)?;
        let num_threads = if threads_input.trim().is_empty() {
            4
        } else {
            threads_input
                .trim()
                .parse::<usize>()
                .map_err(|_| "无效的线程数")?
        };

        print!("请输入下载目录 [./himawari_data]: ");
        io::stdout().flush()?;
        let mut base_path = String::new();
        io::stdin().read_line(&mut base_path)?;
        let base_path = if base_path.trim().is_empty() {
            "./himawari_data".to_string()
        } else {
            base_path.trim().to_string()
        };

        Ok(Config {
            server: ServerConfig {
                host,
                username,
                password,
                port,
            },
            download: DownloadConfig {
                num_threads,
                base_path,
                organize_by_time: true,
                keep_original_structure: false,
            },
        })
    }

    /// 保存配置到文件
    pub fn save_to_file(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let config_content = toml::to_string_pretty(self)?;
        fs::write(path, config_content)?;
        Ok(())
    }

    /// 验证配置
    pub fn validate(&self) -> Result<(), String> {
        if self.server.host.is_empty() {
            return Err("服务器地址不能为空".to_string());
        }
        if self.server.username.is_empty() {
            return Err("用户名不能为空".to_string());
        }
        if self.server.password.is_empty() {
            return Err("密码不能为空".to_string());
        }
        if self.download.num_threads == 0 {
            return Err("线程数必须大于0".to_string());
        }
        Ok(())
    }

    /// 获取完整的主机地址（包含端口）
    pub fn get_host_with_port(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }
}
