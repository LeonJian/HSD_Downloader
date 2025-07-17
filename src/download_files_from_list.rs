pub mod download_files {
    use chrono::NaiveDateTime;
    use ssh2::Session;
    use std::fs::{self, File};
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};
    use std::thread;

    /// 本地文件存储结构
    #[derive(Debug, Clone)]
    pub struct LocalFileStorage {
        pub base_path: PathBuf,
        pub organize_by_time: bool,
    }

    impl LocalFileStorage {
        pub fn new(base_path: &str) -> Self {
            Self {
                base_path: PathBuf::from(base_path),
                organize_by_time: true,
            }
        }

        pub fn with_time_organization(mut self, organize_by_time: bool) -> Self {
            self.organize_by_time = organize_by_time;
            self
        }

        /// 生成本地文件路径
        pub fn generate_local_path(&self, remote_path: &str) -> PathBuf {
            let filename = Path::new(remote_path)
                .file_name()
                .unwrap()
                .to_string_lossy();

            if self.organize_by_time {
                if let Some(parts) = self.parse_filename(&filename) {
                    return self
                        .base_path
                        .join(&parts.year)
                        .join(&parts.month)
                        .join(&parts.day)
                        .join(&parts.hour)
                        .join(filename.as_ref());
                }
            }

            self.base_path.join(filename.as_ref())
        }

        fn parse_filename(&self, filename: &str) -> Option<FilenameParts> {
            // HS_H09_20250717_0900_B03_FLDK_R05_S0101.DAT.bz2
            let parts: Vec<&str> = filename.split('_').collect();
            if parts.len() >= 4 {
                let datetime_str = parts[2];
                let time_str = parts[3];

                if datetime_str.len() == 8 && time_str.len() == 4 {
                    return Some(FilenameParts {
                        year: datetime_str[0..4].to_string(),
                        month: datetime_str[4..6].to_string(),
                        day: datetime_str[6..8].to_string(),
                        hour: time_str[0..2].to_string(),
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

    /// 先下载到内存，确认完毕后再保存到磁盘
    fn download_and_save_file(
        sftp: &ssh2::Sftp,
        remote_path: &str,
        local_storage: &LocalFileStorage,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        println!("正在下载: {}", remote_path);

        let local_path = local_storage.generate_local_path(remote_path);

        // 检查文件是否已经存在
        if local_path.exists() {
            println!("文件已存在，跳过: {}", local_path.display());
            return Ok(0);
        }

        // 第一步：先下载到内存
        let mut remote_file = sftp.open(Path::new(remote_path))?;
        let mut buffer = Vec::new();

        // 读取整个文件到内存
        remote_file.read_to_end(&mut buffer)?;
        let total_bytes = buffer.len() as u64;

        println!(
            "文件已完全下载到内存: {} ({} 字节)",
            remote_path, total_bytes
        );

        // 第二步：确认下载完毕后，创建目录并保存到磁盘
        if let Some(parent) = local_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // 一次性写入磁盘
        let mut local_file = File::create(&local_path)?;
        local_file.write_all(&buffer)?;
        local_file.flush()?;

        println!("完成保存: {} ({} 字节)", local_path.display(), total_bytes);

        Ok(total_bytes)
    }

    /// 流式下载版本（可选用）- 边下载边写入磁盘
    #[allow(dead_code)]
    fn download_and_save_file_streaming(
        sftp: &ssh2::Sftp,
        remote_path: &str,
        local_storage: &LocalFileStorage,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        println!("正在流式下载: {}", remote_path);

        let local_path = local_storage.generate_local_path(remote_path);

        // 检查文件是否已经存在
        if local_path.exists() {
            println!("文件已存在，跳过: {}", local_path.display());
            return Ok(0);
        }

        if let Some(parent) = local_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut remote_file = sftp.open(Path::new(remote_path))?;
        let mut local_file = File::create(&local_path)?;

        let mut buffer = [0; 8192];
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
            "完成流式下载: {} ({} 字节)",
            local_path.display(),
            total_bytes
        );

        Ok(total_bytes)
    }

    /// 读取远程目录并筛选FLDK文件
    fn list_fldk_files_in_directory(
        sftp: &ssh2::Sftp,
        remote_dir: &str,
        target_time: &NaiveDateTime,
        bands: &[String],
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut fldk_files = Vec::new();

        // 读取目录内容
        let dir_entries = sftp.readdir(Path::new(remote_dir))?;

        let target_datetime_str = target_time.format("%Y%m%d_%H%M").to_string();

        for (path, _stat) in dir_entries {
            if let Some(filename) = path.file_name() {
                let filename_str = filename.to_string_lossy();

                // 筛选FLDK文件
                if filename_str.contains("FLDK")
                    && filename_str.contains(&target_datetime_str)
                    && filename_str.ends_with(".DAT.bz2")
                {
                    // 检查是否包含所需波段
                    if bands.is_empty() || bands.iter().any(|band| filename_str.contains(band)) {
                        fldk_files.push(path.to_string_lossy().to_string());
                    }
                }
            }
        }

        Ok(fldk_files)
    }

    /// 获取指定时间的远程目录路径
    fn get_remote_directory_path(datetime: &NaiveDateTime) -> String {
        format!(
            "/jma/hsd/{}/{}/{}/",
            datetime.format("%Y%m"), // 202507
            datetime.format("%d"),   // 17
            datetime.format("%H")
        ) // 09
    }

    /// 收集所有要下载的文件列表
    fn collect_all_files(
        download_list: &[NaiveDateTime],
        bands: &[String],
        host: &str,
        username: &str,
        password: &str,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        println!("开始收集所有需要下载的文件列表...");

        // 建立连接
        let tcp = TcpStream::connect(host)?;
        let mut sess = Session::new().unwrap();
        sess.set_tcp_stream(tcp);
        sess.handshake()?;
        sess.userauth_password(username, password)?;
        let sftp = sess.sftp()?;

        let mut all_files = Vec::new();

        for datetime in download_list {
            let remote_dir = get_remote_directory_path(datetime);

            match list_fldk_files_in_directory(&sftp, &remote_dir, datetime, bands) {
                Ok(files) => {
                    println!("在 {} 找到 {} 个文件", remote_dir, files.len());
                    all_files.extend(files);
                }
                Err(e) => {
                    eprintln!("读取目录失败 {}: {}", remote_dir, e);
                }
            }
        }

        println!("总共收集到 {} 个文件待下载", all_files.len());
        Ok(all_files)
    }

    /// 多线程流式下载FLDK文件 - 优化版
    pub fn download_fldk_files_streaming(
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

        if !bands.is_empty() {
            println!("筛选波段: {:?}", bands);
        } else {
            println!("下载所有FLDK文件");
        }

        println!("准备下载 {} 个时间点的FLDK数据", download_list.len());

        // 第一步：收集所有要下载的文件
        let all_files = collect_all_files(&download_list, &bands, host, username, password)?;

        if all_files.is_empty() {
            println!("没有找到符合条件的文件");
            return Ok(DownloadStats::new());
        }

        // 第二步：将文件分配给线程
        let files_per_thread = (all_files.len() + num_threads - 1) / num_threads;
        let mut distributed_files = Vec::new();

        for i in 0..num_threads {
            let start = i * files_per_thread;
            let end = ((i + 1) * files_per_thread).min(all_files.len());
            if start < all_files.len() {
                distributed_files.push(all_files[start..end].to_vec());
            }
        }

        // 创建共享统计信息
        let total_stats = Arc::new(Mutex::new(DownloadStats::new()));
        let mut handles = Vec::new();

        // 为每个线程创建任务
        for (thread_id, file_list) in distributed_files.into_iter().enumerate() {
            if file_list.is_empty() {
                continue;
            }

            let stats_clone = Arc::clone(&total_stats);
            let host = host.to_string();
            let username = username.to_string();
            let password = password.to_string();
            let storage_clone = local_storage.clone();

            let handle = thread::spawn(move || {
                println!("线程 {} 开始处理 {} 个文件", thread_id, file_list.len());

                // 建立连接
                let tcp = match TcpStream::connect(&host) {
                    Ok(tcp) => tcp,
                    Err(e) => {
                        eprintln!("线程 {} 连接失败: {}", thread_id, e);
                        return;
                    }
                };

                let mut sess = Session::new().unwrap();
                sess.set_tcp_stream(tcp);

                if let Err(e) = sess.handshake() {
                    eprintln!("线程 {} 握手失败: {}", thread_id, e);
                    return;
                }

                if let Err(e) = sess.userauth_password(&username, &password) {
                    eprintln!("线程 {} 认证失败: {}", thread_id, e);
                    return;
                }

                let sftp = match sess.sftp() {
                    Ok(sftp) => sftp,
                    Err(e) => {
                        eprintln!("线程 {} SFTP初始化失败: {}", thread_id, e);
                        return;
                    }
                };

                let mut thread_stats = DownloadStats::new();
                thread_stats.total_files = file_list.len();

                // 下载分配给该线程的所有文件
                for file_path in file_list {
                    match download_and_save_file(&sftp, &file_path, &storage_clone) {
                        Ok(bytes) => {
                            thread_stats.downloaded_files += 1;
                            thread_stats.total_bytes += bytes;
                        }
                        Err(e) => {
                            eprintln!("线程 {} 下载失败 {}: {}", thread_id, file_path, e);
                            thread_stats.failed_files += 1;
                        }
                    }
                }

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
            });

            handles.push(handle);
        }

        // 等待所有线程完成
        for handle in handles {
            handle
                .join()
                .map_err(|e| format!("线程加入失败: {:?}", e))?;
        }

        let final_stats = Arc::try_unwrap(total_stats).unwrap().into_inner().unwrap();

        println!("下载完成统计:");
        println!("  总文件数: {}", final_stats.total_files);
        println!("  成功下载: {}", final_stats.downloaded_files);
        println!("  失败文件: {}", final_stats.failed_files);
        println!("  总字节数: {}", final_stats.total_bytes);

        Ok(final_stats)
    }

    /// 下载可见光波段的FLDK文件
    pub fn download_visible_bands_streaming(
        download_list: Vec<NaiveDateTime>,
        num_threads: usize,
        host: &str,
        username: &str,
        password: &str,
        local_storage: LocalFileStorage,
    ) -> Result<DownloadStats, Box<dyn std::error::Error>> {
        let visible_bands = vec!["B01".to_string(), "B02".to_string(), "B03".to_string()];

        println!("开始下载可见光波段FLDK文件 (B01-B03)");

        download_fldk_files_streaming(
            download_list,
            visible_bands,
            num_threads,
            host,
            username,
            password,
            local_storage,
        )
    }

    /// 下载所有波段的FLDK文件
    pub fn download_all_bands_streaming(
        download_list: Vec<NaiveDateTime>,
        num_threads: usize,
        host: &str,
        username: &str,
        password: &str,
        local_storage: LocalFileStorage,
    ) -> Result<DownloadStats, Box<dyn std::error::Error>> {
        println!("开始下载所有波段FLDK文件");

        download_fldk_files_streaming(
            download_list,
            vec![], // 空列表表示下载所有文件
            num_threads,
            host,
            username,
            password,
            local_storage,
        )
    }

    /// 下载单个波段的FLDK文件
    pub fn download_single_band_streaming(
        download_list: Vec<NaiveDateTime>,
        band: &str,
        num_threads: usize,
        host: &str,
        username: &str,
        password: &str,
        local_storage: LocalFileStorage,
    ) -> Result<DownloadStats, Box<dyn std::error::Error>> {
        println!("开始下载波段 {} 的FLDK文件", band);

        download_fldk_files_streaming(
            download_list,
            vec![band.to_string()],
            num_threads,
            host,
            username,
            password,
            local_storage,
        )
    }
}
