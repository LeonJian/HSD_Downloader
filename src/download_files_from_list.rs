pub mod download_files {
    use chrono::NaiveDateTime;
    use ssh2::Session;
    use std::collections::HashSet;
    use std::fs::{self, OpenOptions};
    use std::io::{Read, Seek, SeekFrom, Write};
    use std::net::TcpStream;
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::{Duration, Instant};

    /// 下载状态
    #[derive(Debug, Clone, PartialEq)]
    pub enum DownloadStatus {
        NotStarted,
        Downloading,
        Completed,
        Failed,
    }

    /// 文件下载记录
    #[derive(Debug, Clone)]
    pub struct FileDownloadRecord {
        pub remote_path: String,
        pub local_path: PathBuf,
        pub temp_path: PathBuf,
        pub expected_size: Option<u64>,
        pub downloaded_size: u64,
        pub status: DownloadStatus,
        pub retry_count: usize,
        pub last_modified: Option<String>,
    }

    /// 本地文件存储结构
    #[derive(Debug, Clone)]
    pub struct LocalFileStorage {
        pub base_path: PathBuf,
        pub organize_by_time: bool,
        pub temp_suffix: String,
    }

    impl LocalFileStorage {
        pub fn new(base_path: &str) -> Self {
            Self {
                base_path: PathBuf::from(base_path),
                organize_by_time: true,
                temp_suffix: ".downloading".to_string(),
            }
        }

        pub fn with_time_organization(mut self, organize_by_time: bool) -> Self {
            self.organize_by_time = organize_by_time;
            self
        }

        pub fn with_temp_suffix(mut self, suffix: &str) -> Self {
            self.temp_suffix = suffix.to_string();
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

        /// 生成临时文件路径
        pub fn generate_temp_path(&self, local_path: &Path) -> PathBuf {
            let mut temp_path = local_path.to_path_buf();
            let mut filename = temp_path.file_name().unwrap().to_string_lossy().to_string();
            filename.push_str(&self.temp_suffix);
            temp_path.set_file_name(filename);
            temp_path
        }

        /// 清理未完成的下载文件
        pub fn cleanup_incomplete_downloads(
            &self,
        ) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
            let mut incomplete_files = Vec::new();
            self.cleanup_directory(&self.base_path, &mut incomplete_files)?;

            if !incomplete_files.is_empty() {
                println!("发现 {} 个未完成的下载文件:", incomplete_files.len());
                for file in &incomplete_files {
                    println!("  删除: {}", file.display());
                    if let Err(e) = fs::remove_file(file) {
                        eprintln!("删除文件失败 {}: {}", file.display(), e);
                    }
                }
            }

            Ok(incomplete_files)
        }

        fn cleanup_directory(
            &self,
            dir: &Path,
            incomplete_files: &mut Vec<PathBuf>,
        ) -> Result<(), Box<dyn std::error::Error>> {
            if !dir.exists() {
                return Ok(());
            }

            let entries = fs::read_dir(dir)?;
            for entry in entries {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    self.cleanup_directory(&path, incomplete_files)?;
                } else if let Some(filename) = path.file_name() {
                    let filename_str = filename.to_string_lossy();
                    if filename_str.ends_with(&self.temp_suffix) {
                        incomplete_files.push(path);
                    }
                }
            }

            Ok(())
        }

        /// 检查波段数据完整性
        pub fn check_band_completeness(
            &self,
            download_list: &[NaiveDateTime],
            bands: &[String],
        ) -> BandCompletenessReport {
            let mut report = BandCompletenessReport::new();

            for datetime in download_list {
                let mut time_report = TimeSlotReport {
                    datetime: *datetime,
                    bands: Vec::new(),
                };

                for band in bands {
                    let expected_filename = format!(
                        "HS_H09_{}_FLDK_R05_S0101.DAT.bz2",
                        format!("{}{}", datetime.format("%Y%m%d_%H%M"), band)
                    );

                    let local_path = self.generate_local_path(&expected_filename);
                    let exists = local_path.exists();
                    let size = if exists {
                        fs::metadata(&local_path).map(|m| m.len()).unwrap_or(0)
                    } else {
                        0
                    };

                    time_report.bands.push(BandStatus {
                        band: band.clone(),
                        exists,
                        size,
                        path: local_path,
                    });
                }

                report.time_slots.push(time_report);
            }

            report
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

    /// 波段状态
    #[derive(Debug, Clone)]
    pub struct BandStatus {
        pub band: String,
        pub exists: bool,
        pub size: u64,
        pub path: PathBuf,
    }

    /// 时间段报告
    #[derive(Debug, Clone)]
    pub struct TimeSlotReport {
        pub datetime: NaiveDateTime,
        pub bands: Vec<BandStatus>,
    }

    /// 波段完整性报告
    #[derive(Debug, Clone)]
    pub struct BandCompletenessReport {
        pub time_slots: Vec<TimeSlotReport>,
    }

    impl BandCompletenessReport {
        pub fn new() -> Self {
            Self {
                time_slots: Vec::new(),
            }
        }

        pub fn print_report(&self) {
            println!("=== 波段数据完整性报告 ===");
            for slot in &self.time_slots {
                println!("时间: {}", slot.datetime.format("%Y-%m-%d %H:%M"));
                for band in &slot.bands {
                    let status = if band.exists { "✓" } else { "✗" };
                    println!("  {} {}: {} bytes", status, band.band, band.size);
                }
            }
        }
    }

    /// 下载统计信息
    #[derive(Debug, Clone)]
    pub struct DownloadStats {
        pub total_files: usize,
        pub downloaded_files: usize,
        pub failed_files: usize,
        pub skipped_files: usize,
        pub total_bytes: u64,
        pub elapsed_time: Duration,
    }

    impl DownloadStats {
        pub fn new() -> Self {
            Self {
                total_files: 0,
                downloaded_files: 0,
                failed_files: 0,
                skipped_files: 0,
                total_bytes: 0,
                elapsed_time: Duration::from_secs(0),
            }
        }

        pub fn print_summary(&self) {
            println!("=== 下载统计摘要 ===");
            println!("总文件数: {}", self.total_files);
            println!("成功下载: {}", self.downloaded_files);
            println!("跳过文件: {}", self.skipped_files);
            println!("失败文件: {}", self.failed_files);
            println!("总下载量: {} MB", self.total_bytes / 1024 / 1024);
            println!("耗时: {:?}", self.elapsed_time);
            if self.elapsed_time.as_secs() > 0 {
                let speed =
                    self.total_bytes as f64 / self.elapsed_time.as_secs_f64() / 1024.0 / 1024.0;
                println!("平均速度: {:.2} MB/s", speed);
            }
        }
    }

    /// 边下载边写入磁盘的安全版本
    fn download_and_save_file_streaming(
        sftp: &ssh2::Sftp,
        remote_path: &str,
        local_storage: &LocalFileStorage,
        max_retries: usize,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        let local_path = local_storage.generate_local_path(remote_path);
        let temp_path = local_storage.generate_temp_path(&local_path);

        // 检查文件是否已经存在并且完整
        if local_path.exists() {
            let local_size = fs::metadata(&local_path)?.len();
            if local_size > 0 {
                println!(
                    "文件已存在，跳过: {} ({} bytes)",
                    local_path.display(),
                    local_size
                );
                return Ok(0);
            }
        }

        // 创建目录
        if let Some(parent) = local_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut retry_count = 0;
        let mut last_error = None;

        while retry_count <= max_retries {
            match download_file_with_resume(sftp, remote_path, &temp_path, &local_path) {
                Ok(bytes) => {
                    println!("完成下载: {} ({} bytes)", local_path.display(), bytes);
                    return Ok(bytes);
                }
                Err(e) => {
                    last_error = Some(e);
                    retry_count += 1;
                    if retry_count <= max_retries {
                        println!(
                            "下载失败，重试 {}/{}: {}",
                            retry_count, max_retries, remote_path
                        );
                        thread::sleep(Duration::from_secs(2));
                    }
                }
            }
        }

        Err(format!("下载失败，已重试 {} 次: {:?}", max_retries, last_error).into())
    }

    /// 支持断点续传的下载函数
    fn download_file_with_resume(
        sftp: &ssh2::Sftp,
        remote_path: &str,
        temp_path: &Path,
        final_path: &Path,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        // 获取远程文件信息
        let remote_stat = sftp.stat(Path::new(remote_path))?;
        let remote_size = remote_stat.size.unwrap_or(0);

        // 检查是否存在临时文件
        let mut start_pos = 0u64;
        if temp_path.exists() {
            let temp_size = fs::metadata(temp_path)?.len();
            if temp_size < remote_size {
                start_pos = temp_size;
                println!("断点续传: {} (从 {} 字节开始)", remote_path, start_pos);
            } else {
                fs::remove_file(temp_path)?;
            }
        }

        // 打开远程文件
        let mut remote_file = sftp.open(Path::new(remote_path))?;
        if start_pos > 0 {
            remote_file.seek(SeekFrom::Start(start_pos))?;
        }

        // 打开本地临时文件
        let mut local_file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(start_pos > 0)
            .truncate(start_pos == 0)
            .open(temp_path)?;

        // 使用缓冲区进行流式传输
        let mut buffer = [0u8; 32768]; // 32KB 缓冲区
        let mut total_bytes = start_pos;
        let mut last_report_time = Instant::now();

        loop {
            match remote_file.read(&mut buffer) {
                Ok(0) => break, // EOF
                Ok(bytes_read) => {
                    local_file.write_all(&buffer[..bytes_read])?;
                    total_bytes += bytes_read as u64;

                    // 定期报告进度
                    if last_report_time.elapsed() > Duration::from_secs(5) {
                        let progress = (total_bytes as f64 / remote_size as f64) * 100.0;
                        println!(
                            "下载进度: {:.1}% ({}/{} bytes)",
                            progress, total_bytes, remote_size
                        );
                        last_report_time = Instant::now();
                    }
                }
                Err(e) => {
                    return Err(format!("读取远程文件失败: {}", e).into());
                }
            }
        }

        // 确保数据写入磁盘
        local_file.flush()?;
        local_file.sync_all()?;

        // 验证文件大小
        if total_bytes != remote_size {
            return Err(format!(
                "文件大小不匹配: 预期 {} 字节，实际 {} 字节",
                remote_size, total_bytes
            )
            .into());
        }

        // 将临时文件移动到最终位置
        fs::rename(temp_path, final_path)?;

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
            datetime.format("%H")    // 09
        )
    }

    /// 收集所有要下载的文件列表并过滤已存在的文件
    fn collect_files_to_download(
        download_list: &[NaiveDateTime],
        bands: &[String],
        host: &str,
        username: &str,
        password: &str,
        local_storage: &LocalFileStorage,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        println!("开始收集需要下载的文件列表...");

        // 建立连接
        let tcp = TcpStream::connect(host)?;
        let mut sess = Session::new().unwrap();
        sess.set_tcp_stream(tcp);
        sess.handshake()?;
        sess.userauth_password(username, password)?;
        let sftp = sess.sftp()?;

        let mut files_to_download = Vec::new();
        let mut existing_files = HashSet::new();

        for datetime in download_list {
            let remote_dir = get_remote_directory_path(datetime);

            match list_fldk_files_in_directory(&sftp, &remote_dir, datetime, bands) {
                Ok(files) => {
                    println!("在 {} 找到 {} 个文件", remote_dir, files.len());

                    for file in files {
                        let local_path = local_storage.generate_local_path(&file);

                        // 检查文件是否已存在且完整
                        if local_path.exists() {
                            if let Ok(metadata) = fs::metadata(&local_path) {
                                if metadata.len() > 0 {
                                    existing_files.insert(file);
                                    continue;
                                }
                            }
                        }

                        files_to_download.push(file);
                    }
                }
                Err(e) => {
                    eprintln!("读取目录失败 {}: {}", remote_dir, e);
                }
            }
        }

        println!("已存在文件: {} 个", existing_files.len());
        println!("需要下载: {} 个", files_to_download.len());

        Ok(files_to_download)
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
        let start_time = Instant::now();

        if download_list.is_empty() {
            println!("下载列表为空，跳过下载");
            return Ok(DownloadStats::new());
        }

        // 清理未完成的下载
        println!("清理未完成的下载文件...");
        let cleanup_result = local_storage.cleanup_incomplete_downloads()?;
        if !cleanup_result.is_empty() {
            println!("已清理 {} 个未完成的下载文件", cleanup_result.len());
        }

        // 检查波段数据完整性
        if !bands.is_empty() {
            println!("检查波段数据完整性...");
            let report = local_storage.check_band_completeness(&download_list, &bands);
            report.print_report();
        }

        if !bands.is_empty() {
            println!("筛选波段: {:?}", bands);
        } else {
            println!("下载所有FLDK文件");
        }

        println!("准备下载 {} 个时间点的FLDK数据", download_list.len());

        // 收集需要下载的文件
        let files_to_download = collect_files_to_download(
            &download_list,
            &bands,
            host,
            username,
            password,
            &local_storage,
        )?;

        if files_to_download.is_empty() {
            println!("没有需要下载的文件");
            return Ok(DownloadStats::new());
        }

        // 将文件分配给线程
        let files_per_thread = (files_to_download.len() + num_threads - 1) / num_threads;
        let mut distributed_files = Vec::new();

        for i in 0..num_threads {
            let start = i * files_per_thread;
            let end = ((i + 1) * files_per_thread).min(files_to_download.len());
            if start < files_to_download.len() {
                distributed_files.push(files_to_download[start..end].to_vec());
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
                    match download_and_save_file_streaming(&sftp, &file_path, &storage_clone, 3) {
                        Ok(bytes) => {
                            if bytes > 0 {
                                thread_stats.downloaded_files += 1;
                                thread_stats.total_bytes += bytes;
                            } else {
                                thread_stats.skipped_files += 1;
                            }
                        }
                        Err(e) => {
                            eprintln!("线程 {} 下载失败 {}: {}", thread_id, file_path, e);
                            thread_stats.failed_files += 1;
                        }
                    }
                }

                println!(
                    "线程 {} 完成，成功: {}, 跳过: {}, 失败: {}, 总字节: {}",
                    thread_id,
                    thread_stats.downloaded_files,
                    thread_stats.skipped_files,
                    thread_stats.failed_files,
                    thread_stats.total_bytes
                );

                // 合并统计信息
                let mut total_stats = stats_clone.lock().unwrap();
                total_stats.total_files += thread_stats.total_files;
                total_stats.downloaded_files += thread_stats.downloaded_files;
                total_stats.skipped_files += thread_stats.skipped_files;
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

        let mut final_stats = Arc::try_unwrap(total_stats).unwrap().into_inner().unwrap();
        final_stats.elapsed_time = start_time.elapsed();

        final_stats.print_summary();

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
