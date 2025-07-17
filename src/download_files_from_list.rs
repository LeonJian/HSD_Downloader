pub mod download_files {
    use chrono::NaiveDateTime;
    use ssh2::Session;
    use std::fs::{self, File};
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};
    use std::thread;

    // Himawari 9 频段定义
    const HIMAWARI_BANDS: [&str; 16] = [
        "B01", "B02", "B03", "B04", "B05", "B06", "B07", "B08", "B09", "B10", "B11", "B12", "B13",
        "B14", "B15", "B16",
    ];

    // 每个波段的10个分段文件标识符
    const SEGMENT_IDS: [&str; 10] = [
        "S0110", "S0210", "S0310", "S0410", "S0510", "S0610", "S0710", "S0810", "S0910", "S1010",
    ];

    /// 本地文件存储结构
    #[derive(Debug, Clone)]
    pub struct LocalFileStorage {
        pub base_path: PathBuf,
        pub organize_by_time: bool, // true: 按时间组织，false: 按波段组织
        pub keep_original_structure: bool, // 是否保持原始目录结构
    }

    impl LocalFileStorage {
        pub fn new(base_path: &str) -> Self {
            Self {
                base_path: PathBuf::from(base_path),
                organize_by_time: true,
                keep_original_structure: false,
            }
        }

        pub fn with_time_organization(mut self, organize_by_time: bool) -> Self {
            self.organize_by_time = organize_by_time;
            self
        }

        pub fn with_original_structure(mut self, keep_original: bool) -> Self {
            self.keep_original_structure = keep_original;
            self
        }

        /// 根据远程文件路径生成本地文件路径
        pub fn generate_local_path(&self, remote_path: &str) -> PathBuf {
            if self.keep_original_structure {
                // 保持原始目录结构
                let relative_path = remote_path.trim_start_matches('/');
                self.base_path.join(relative_path)
            } else {
                // 自定义组织结构
                let filename = Path::new(remote_path)
                    .file_name()
                    .unwrap()
                    .to_string_lossy();

                if self.organize_by_time {
                    // 按时间组织: base_path/YYYY/MM/DD/HH/filename
                    self.extract_time_path_from_filename(&filename)
                } else {
                    // 按波段组织: base_path/BXX/YYYY/MM/DD/HH/filename
                    self.extract_band_path_from_filename(&filename)
                }
            }
        }

        fn extract_time_path_from_filename(&self, filename: &str) -> PathBuf {
            // 解析文件名: HS_H09_20250717_0900_B01_FLDK_R10_S0110.DAT.bz2
            if let Some(parts) = self.parse_filename(filename) {
                self.base_path
                    .join(&parts.year)
                    .join(&parts.month)
                    .join(&parts.day)
                    .join(&parts.hour)
                    .join(filename)
            } else {
                // 如果解析失败，放在根目录
                self.base_path.join(filename)
            }
        }

        fn extract_band_path_from_filename(&self, filename: &str) -> PathBuf {
            if let Some(parts) = self.parse_filename(filename) {
                self.base_path
                    .join(&parts.band)
                    .join(&parts.year)
                    .join(&parts.month)
                    .join(&parts.day)
                    .join(&parts.hour)
                    .join(filename)
            } else {
                self.base_path.join(filename)
            }
        }

        fn parse_filename(&self, filename: &str) -> Option<FilenameParts> {
            // HS_H09_20250717_0900_B01_FLDK_R10_S0110.DAT.bz2
            let parts: Vec<&str> = filename.split('_').collect();
            if parts.len() >= 5 {
                let datetime_str = parts[2];
                let time_str = parts[3];
                let band = parts[4];

                if datetime_str.len() == 8 && time_str.len() == 4 {
                    return Some(FilenameParts {
                        year: datetime_str[0..4].to_string(),
                        month: datetime_str[4..6].to_string(),
                        day: datetime_str[6..8].to_string(),
                        hour: time_str[0..2].to_string(),
                        band: band.to_string(),
                    });
                }
            }
            None
        }
    }

    #[derive(Debug)]
    struct FilenameParts {
        year: String,
        month: String,
        day: String,
        hour: String,
        band: String,
    }

    /// 下载统计信息
    #[derive(Debug, Clone)]
    pub struct DownloadStats {
        pub total_files: usize,
        pub downloaded_files: usize,
        pub failed_files: usize,
        pub total_bytes: u64,
    }

    impl DownloadStats {
        pub fn new() -> Self {
            Self {
                total_files: 0,
                downloaded_files: 0,
                failed_files: 0,
                total_bytes: 0,
            }
        }
    }

    fn distribute_download_to_threads(
        download_list: Vec<NaiveDateTime>,
        num_threads: usize,
    ) -> Result<Vec<Vec<NaiveDateTime>>, Box<dyn std::error::Error>> {
        if num_threads == 0 {
            Err("Number of threads must be greater than 0")?;
        }

        let mut result: Vec<Vec<NaiveDateTime>> = vec![Vec::new(); num_threads];

        for (i, time) in download_list.into_iter().enumerate() {
            let thread_index = i % num_threads;
            result[thread_index].push(time);
        }
        Ok(result)
    }

    /// 流式下载单个文件并立即写入磁盘
    fn download_and_save_file(
        sftp: &ssh2::Sftp,
        remote_path: &str,
        local_storage: &LocalFileStorage,
    ) -> Result<(String, u64), Box<dyn std::error::Error>> {
        println!("正在下载: {}", remote_path);

        // 生成本地文件路径
        let local_path = local_storage.generate_local_path(remote_path);

        // 确保目录存在
        if let Some(parent) = local_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // 打开远程文件
        let mut remote_file = sftp.open(Path::new(remote_path))?;

        // 创建本地文件
        let mut local_file = File::create(&local_path)?;

        // 流式传输数据
        let mut buffer = [0; 8192]; // 8KB缓冲区
        let mut total_bytes = 0u64;

        loop {
            let bytes_read = remote_file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }

            local_file.write_all(&buffer[..bytes_read])?;
            total_bytes += bytes_read as u64;
        }

        local_file.flush()?;

        println!(
            "完成下载: {} -> {} ({} 字节)",
            remote_path,
            local_path.display(),
            total_bytes
        );

        Ok((local_path.to_string_lossy().to_string(), total_bytes))
    }

    /// 流式下载多个文件
    fn download_files_streaming(
        host: &str,
        username: &str,
        password: &str,
        remote_file_paths: Vec<String>,
        local_storage: &LocalFileStorage,
    ) -> Result<DownloadStats, Box<dyn std::error::Error>> {
        let tcp = TcpStream::connect(host)?;
        let mut sess = Session::new()?;
        sess.set_tcp_stream(tcp);
        sess.handshake()?;
        sess.userauth_password(username, password)?;
        let sftp = sess.sftp()?;

        let mut stats = DownloadStats::new();
        stats.total_files = remote_file_paths.len();

        for remote_path in remote_file_paths {
            match download_and_save_file(&sftp, &remote_path, local_storage) {
                Ok((local_path, bytes)) => {
                    stats.downloaded_files += 1;
                    stats.total_bytes += bytes;
                }
                Err(e) => {
                    eprintln!("下载失败 {}: {}", remote_path, e);
                    stats.failed_files += 1;
                }
            }
        }

        Ok(stats)
    }

    /// 为指定波段和时间生成所有10个分段文件的路径
    fn generate_band_segment_paths(datetime: &NaiveDateTime, band: &str) -> Vec<String> {
        let mut file_paths = Vec::new();

        let year = datetime.format("%Y").to_string();
        let month = datetime.format("%m").to_string();
        let day = datetime.format("%d").to_string();
        let hour = datetime.format("%H").to_string();
        let datetime_str = datetime.format("%Y%m%d_%H%M").to_string();

        for segment_id in SEGMENT_IDS.iter() {
            let filename = format!(
                "HS_H09_{}_{}_FLDK_R10_{}.DAT.bz2",
                datetime_str, band, segment_id
            );
            let full_path = format!("/jma/hsd/{}/{}/{}/{}/{}", year, month, day, hour, filename);
            file_paths.push(full_path);
        }

        file_paths
    }

    /// 为指定时间和波段列表生成所有文件路径
    fn generate_himawari_file_paths(datetime: &NaiveDateTime, bands: &[String]) -> Vec<String> {
        let mut all_file_paths = Vec::new();

        for band in bands {
            let band_paths = generate_band_segment_paths(datetime, band);
            all_file_paths.extend(band_paths);
        }

        all_file_paths
    }

    /// 多线程流式下载Himawari文件
    pub fn download_himawari_files_streaming_multi_thread(
        download_list: Vec<NaiveDateTime>,
        bands: Vec<String>,
        num_threads: usize,
        host: &str,
        username: &str,
        password: &str,
        local_storage: LocalFileStorage,
    ) -> Result<DownloadStats, Box<dyn std::error::Error>> {
        if download_list.is_empty() {
            println!("下载列表为空，跳过下载");
            return Ok(DownloadStats::new());
        }

        if bands.is_empty() {
            println!("未指定波段，跳过下载");
            return Ok(DownloadStats::new());
        }

        // 验证波段
        for band in &bands {
            if !HIMAWARI_BANDS.contains(&band.as_str()) {
                return Err(format!("无效的波段: {}", band).into());
            }
        }

        println!(
            "准备下载 {} 个时间点，{} 个波段，每个波段10个文件",
            download_list.len(),
            bands.len()
        );

        // 分配任务到线程
        let distributed_tasks = distribute_download_to_threads(download_list, num_threads)?;

        // 创建共享统计信息
        let total_stats = Arc::new(Mutex::new(DownloadStats::new()));
        let mut handles = Vec::new();

        // 为每个线程创建任务
        for (thread_id, task_list) in distributed_tasks.into_iter().enumerate() {
            if task_list.is_empty() {
                println!("线程 {} 没有任务，跳过", thread_id);
                continue;
            }

            let stats_clone = Arc::clone(&total_stats);
            let host = host.to_string();
            let username = username.to_string();
            let password = password.to_string();
            let bands_clone = bands.clone();
            let storage_clone = local_storage.clone();

            let handle = thread::spawn(move || {
                println!("线程 {} 开始处理 {} 个时间点", thread_id, task_list.len());

                // 为当前线程的所有时间点和波段生成文件路径
                let mut all_file_paths = Vec::new();
                for datetime in &task_list {
                    let file_paths = generate_himawari_file_paths(datetime, &bands_clone);
                    all_file_paths.extend(file_paths);
                }

                println!("线程 {} 将下载 {} 个文件", thread_id, all_file_paths.len());

                // 流式下载文件
                match download_files_streaming(
                    &host,
                    &username,
                    &password,
                    all_file_paths,
                    &storage_clone,
                ) {
                    Ok(thread_stats) => {
                        println!(
                            "线程 {} 完成，成功: {}, 失败: {}, 总字节: {}",
                            thread_id,
                            thread_stats.downloaded_files,
                            thread_stats.failed_files,
                            thread_stats.total_bytes
                        );

                        // 合并统计信息
                        let mut total_stats = stats_clone.lock().unwrap();
                        total_stats.total_files += thread_stats.total_files;
                        total_stats.downloaded_files += thread_stats.downloaded_files;
                        total_stats.failed_files += thread_stats.failed_files;
                        total_stats.total_bytes += thread_stats.total_bytes;
                    }
                    Err(e) => {
                        eprintln!("线程 {} 下载失败: {}", thread_id, e);
                    }
                }
            });

            handles.push(handle);
        }

        // 等待所有线程完成
        for handle in handles {
            handle
                .join()
                .map_err(|e| format!("线程加入失败: {:?}", e))?;
        }

        // 返回统计结果
        let final_stats = Arc::try_unwrap(total_stats).unwrap().into_inner().unwrap();
        Ok(final_stats)
    }

    /// 便捷函数：下载所有波段
    pub fn download_all_bands_streaming(
        download_list: Vec<NaiveDateTime>,
        num_threads: usize,
        host: &str,
        username: &str,
        password: &str,
        local_storage: LocalFileStorage,
    ) -> Result<DownloadStats, Box<dyn std::error::Error>> {
        download_himawari_files_streaming_multi_thread(
            download_list,
            HIMAWARI_BANDS.iter().map(|s| s.to_string()).collect(),
            num_threads,
            host,
            username,
            password,
            local_storage,
        )
    }

    /// 便捷函数：下载单个波段
    pub fn download_single_band_streaming(
        download_list: Vec<NaiveDateTime>,
        band: &str,
        num_threads: usize,
        host: &str,
        username: &str,
        password: &str,
        local_storage: LocalFileStorage,
    ) -> Result<DownloadStats, Box<dyn std::error::Error>> {
        download_himawari_files_streaming_multi_thread(
            download_list,
            vec![band.to_string()],
            num_threads,
            host,
            username,
            password,
            local_storage,
        )
    }

    /// 下载可见光波段（B01-B03）的流式函数
    pub fn download_visible_bands_streaming(
        download_list: Vec<NaiveDateTime>,
        num_threads: usize,
        host: &str,
        username: &str,
        password: &str,
        local_storage: LocalFileStorage,
    ) -> Result<DownloadStats, Box<dyn std::error::Error>> {
        // 可见光波段：B01(0.47μm), B02(0.51μm), B03(0.64μm)
        let visible_bands = vec![
            "B01".to_string(), // 蓝色波段 (0.47 μm)
            "B02".to_string(), // 绿色波段 (0.51 μm)
            "B03".to_string(), // 红色波段 (0.64 μm)
        ];

        println!("开始下载可见光波段 (B01-B03)");
        println!("B01: 蓝色波段 (0.47 μm)");
        println!("B02: 绿色波段 (0.51 μm)");
        println!("B03: 红色波段 (0.64 μm)");

        download_himawari_files_streaming_multi_thread(
            download_list,
            visible_bands,
            num_threads,
            host,
            username,
            password,
            local_storage,
        )
    }

    /// 下载近红外波段（B04-B06）的流式函数
    pub fn download_near_infrared_bands_streaming(
        download_list: Vec<NaiveDateTime>,
        num_threads: usize,
        host: &str,
        username: &str,
        password: &str,
        local_storage: LocalFileStorage,
    ) -> Result<DownloadStats, Box<dyn std::error::Error>> {
        // 近红外波段：B04(0.86μm), B05(1.6μm), B06(2.3μm)
        let near_infrared_bands = vec![
            "B04".to_string(), // 近红外波段 (0.86 μm)
            "B05".to_string(), // 短波红外波段 (1.6 μm)
            "B06".to_string(), // 短波红外波段 (2.3 μm)
        ];

        println!("开始下载近红外波段 (B04-B06)");
        println!("B04: 近红外波段 (0.86 μm)");
        println!("B05: 短波红外波段 (1.6 μm)");
        println!("B06: 短波红外波段 (2.3 μm)");

        download_himawari_files_streaming_multi_thread(
            download_list,
            near_infrared_bands,
            num_threads,
            host,
            username,
            password,
            local_storage,
        )
    }

    /// 下载红外波段（B07-B16）的流式函数
    pub fn download_infrared_bands_streaming(
        download_list: Vec<NaiveDateTime>,
        num_threads: usize,
        host: &str,
        username: &str,
        password: &str,
        local_storage: LocalFileStorage,
    ) -> Result<DownloadStats, Box<dyn std::error::Error>> {
        // 红外波段：B07-B16
        let infrared_bands = vec![
            "B07".to_string(), // 短波红外波段 (3.9 μm)
            "B08".to_string(), // 水蒸气波段 (6.2 μm)
            "B09".to_string(), // 水蒸气波段 (6.9 μm)
            "B10".to_string(), // 水蒸气波段 (7.3 μm)
            "B11".to_string(), // 云顶温度波段 (8.6 μm)
            "B12".to_string(), // 臭氧波段 (9.6 μm)
            "B13".to_string(), // 清洁长波红外波段 (10.4 μm)
            "B14".to_string(), // 长波红外波段 (11.2 μm)
            "B15".to_string(), // 长波红外波段 (12.4 μm)
            "B16".to_string(), // 脏长波红外波段 (13.3 μm)
        ];

        println!("开始下载红外波段 (B07-B16)");

        download_himawari_files_streaming_multi_thread(
            download_list,
            infrared_bands,
            num_threads,
            host,
            username,
            password,
            local_storage,
        )
    }

    /// 下载真彩色RGB所需的波段（B01, B02, B03）
    pub fn download_true_color_bands_streaming(
        download_list: Vec<NaiveDateTime>,
        num_threads: usize,
        host: &str,
        username: &str,
        password: &str,
        local_storage: LocalFileStorage,
    ) -> Result<DownloadStats, Box<dyn std::error::Error>> {
        println!("开始下载真彩色RGB波段 (B01, B02, B03)");
        download_visible_bands_streaming(
            download_list,
            num_threads,
            host,
            username,
            password,
            local_storage,
        )
    }

    /// 下载自然彩色RGB所需的波段（B01, B02, B03, B04）
    pub fn download_natural_color_bands_streaming(
        download_list: Vec<NaiveDateTime>,
        num_threads: usize,
        host: &str,
        username: &str,
        password: &str,
        local_storage: LocalFileStorage,
    ) -> Result<DownloadStats, Box<dyn std::error::Error>> {
        let natural_color_bands = vec![
            "B01".to_string(), // 蓝色
            "B02".to_string(), // 绿色
            "B03".to_string(), // 红色
            "B04".to_string(), // 近红外
        ];

        println!("开始下载自然彩色RGB波段 (B01-B04)");

        download_himawari_files_streaming_multi_thread(
            download_list,
            natural_color_bands,
            num_threads,
            host,
            username,
            password,
            local_storage,
        )
    }
}
