use Himawari_HSD_downloader::download_files_from_list::download_files::{
    LocalFileStorage, download_visible_bands_streaming,
};
use Himawari_HSD_downloader::get_download_time_list::get_download_time_list::get_download_time_list;

fn main() {
    let version = env!("CARGO_PKG_VERSION");
    println!(
        "----------  Himawari HSD Data Downloader Version {}  ----------",
        version
    );

    let download_time_list = get_download_time_list();
    println!("Download Time List: {:?}", download_time_list);

    let storage = LocalFileStorage::new("./himawari_visible")
        .with_time_organization(true) // 按时间组织文件
        .with_original_structure(false);

    let stats = download_visible_bands_streaming(
        download_time_list,
        4,
        "Host",
        "Username",
        "Password",
        storage,
    );
}
