pub mod download_files {
    use chrono::NaiveDateTime;
    use ssh2::Session;
    use std::io::Read;
    use std::net::TcpStream;
    use std::path::Path;

    fn distribute_download_to_threads(
        download_list: Vec<NaiveDateTime>,
        num_threads: usize,
    ) -> Result<Vec<Vec<NaiveDateTime>>, Box<dyn std::error::Error>> {
        if num_threads == 0 {
            Err("Number of threads must be greater than 0")?;
        }

        let mut result: Vec<Vec<NaiveDateTime>> = vec![Vec::new(); num_threads];
        let total = download_list.len();

        for (i, time) in download_list.into_iter().enumerate() {
            // 采用轮询分配法（round-robin）更均衡
            let thread_index = i % num_threads;
            result[thread_index].push(time);
        }
        Ok(result)
    }

    fn download_from_server_to_memory(
        host: &str,
        username: &str,
        password: &str,
        remote_file_path: Vec<&str>,
    ) -> Result<Vec<(String, Vec<u8>)>, Box<dyn std::error::Error>> {
        let tcp = TcpStream::connect(host)?;
        let mut sess = Session::new()?;
        sess.set_tcp_stream(tcp);
        sess.handshake()?;
        sess.userauth_password(username, password)?;
        let sftp = sess.sftp()?;

        let mut result: Vec<(String, Vec<u8>)> = Vec::new();

        for remote_file_path in remote_file_path {
            println!("Downloading from {}", remote_file_path);
            let mut remote_file = sftp.open(Path::new(remote_file_path))?;
            let mut buffer = Vec::new();
            remote_file.read_to_end(&mut buffer)?;
            result.push((remote_file_path.parse().unwrap(), buffer));
        }
        Ok(result)
    }
}
