use Himawari_HSD_downloader::get_download_time_list::get_download_time_list::get_download_time_list;

fn main() {
    let version = env!("CARGO_PKG_VERSION");
    println!(
        "----------  Himawari HSD Data Downloader Version {}  ----------",
        version
    );

    let download_time_list = get_download_time_list();
    println!("Download Time List: {:?}", download_time_list);
}
