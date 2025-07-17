use Himawari_HSD_downloader::download_files_from_list::download_files::{
    LocalFileStorage, download_visible_bands_streaming,
};
use Himawari_HSD_downloader::get_download_time_list::get_download_time_list::get_download_time_list;

mod config;
use config::Config;

fn main() {
    let version = env!("CARGO_PKG_VERSION");
    println!(
        "----------  Himawari HSD Data Downloader Version {}  ----------",
        version
    );

    // 配置文件路径
    let config_path = "config.toml";

    // 加载配置
    let config = match Config::load_or_create(config_path) {
        Ok(config) => config,
        Err(e) => {
            println!("配置加载失败: {}", e);
            println!("是否要交互式设置配置? (y/n): ");

            let mut input = String::new();
            std::io::stdin()
                .read_line(&mut input)
                .expect("读取输入失败");

            if input.trim().to_lowercase() == "y" {
                match Config::interactive_setup() {
                    Ok(config) => {
                        // 保存配置
                        if let Err(e) = config.save_to_file(config_path) {
                            eprintln!("保存配置失败: {}", e);
                        } else {
                            println!("配置已保存到: {}", config_path);
                        }
                        config
                    }
                    Err(e) => {
                        eprintln!("交互式配置失败: {}", e);
                        return;
                    }
                }
            } else {
                println!("请手动编辑配置文件后重新运行程序");
                return;
            }
        }
    };

    // 验证配置
    if let Err(e) = config.validate() {
        eprintln!("配置验证失败: {}", e);
        return;
    }

    println!("使用配置:");
    println!("  服务器: {}", config.get_host_with_port());
    println!("  用户名: {}", config.server.username);
    println!("  线程数: {}", config.download.num_threads);
    println!("  下载目录: {}", config.download.base_path);

    // 获取下载时间列表
    let download_time_list = get_download_time_list();
    println!("下载时间列表: {:?}", download_time_list);

    // 创建本地存储配置
    let storage = LocalFileStorage::new(&config.download.base_path)
        .with_time_organization(config.download.organize_by_time);

    // 执行下载
    println!("开始下载可见光波段数据...");
    match download_visible_bands_streaming(
        download_time_list,
        config.download.num_threads,
        &config.get_host_with_port(),
        &config.server.username,
        &config.server.password,
        storage,
    ) {
        Ok(stats) => {
            println!("下载完成！");
            println!("成功下载: {} 个文件", stats.downloaded_files);
            println!("下载失败: {} 个文件", stats.failed_files);
            println!("总下载量: {} 字节", stats.total_bytes);
        }
        Err(e) => {
            eprintln!("下载失败: {}", e);
        }
    }
}
