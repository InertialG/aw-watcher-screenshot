mod capture;

use image::ImageFormat;
use std::fs;
use tokio::signal;
use tokio::sync::{broadcast, mpsc};
use tracing::{error, info};
use tracing_subscriber;
use uuid::Uuid;

use capture::capture::Capture;
use capture::event::MonitorImageEvent;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    info!("Starting capture service...");

    // 1. 准备环境
    fs::create_dir_all("./images").map_err(|e| {
        error!("Directory creation failed: {}", e);
        e
    })?;

    // 2. 初始化通信管道
    // stop_tx 用于发送退出信号，tx 用于传递图片
    let (stop_tx, _) = broadcast::channel::<bool>(1);
    let (tx, mut rx) = mpsc::channel::<MonitorImageEvent>(100);

    let saver_handler = tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            let path = format!("./images/{}.jpg", Uuid::new_v4());
            tokio::task::spawn_blocking(move || {
                let rgb_image = event.image().to_rgb8();
                match rgb_image.save_with_format(&path, ImageFormat::Jpeg) {
                    Ok(_) => info!("Image saved successfully to {}", path),
                    Err(e) => info!("Error saving image: {}", e),
                }
            });
        }
        info!("Saver task finished.");
    });

    // 4. 初始化并运行捕获器
    let mut capture = Capture::new(tx.clone(), stop_tx.clone());
    match capture.run() {
        Ok(_) => info!("Capture task finished."),
        Err(e) => error!("Error running capture task: {}", e),
    };

    // 5. 等待退出信号 (Ctrl+C)
    signal::ctrl_c().await?;
    info!("Ctrl+C received, shutting down...");

    // 6. 优雅退出流程
    let _ = stop_tx.send(true); // 通知所有截图任务停止

    // 等待所有截图任务停止
    capture.wait().await;
    info!("All capture tasks stopped.");

    // 7. 优雅退出第三步：显式释放掉 main 里的这个 tx
    // 这一点至关重要！如果不 drop(tx)，rx 会以为还有人可能发消息，从而永远等下去
    drop(tx);

    // 8. 优雅退出第四步：等待 Saver 处理完管道里的“存货”
    // 当所有 tx 都被释放，rx.recv() 返回 None，saver_handler 才会自然结束
    saver_handler.await?;
    info!("All images saved. Exit clean.");

    info!("All tasks finished. Bye!");
    Ok(())
}
