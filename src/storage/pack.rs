use anyhow::{Context, Error, Result, anyhow};
use chrono::{DateTime, Utc};
use image::{DynamicImage, ImageFormat};
use std::io::Write;
use std::iter;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tracing::{error, info};

use crate::event::MonitorImageEvent;

// 最终的数据汇（Sink）：内存中的字节数组
type MemBuffer = Vec<u8>;
// 第三层：Age 加密写入器
type AgeWriter = age::stream::StreamWriter<MemBuffer>;
// 第二层：Zstd 压缩写入器
type ZstdWriter = zstd::stream::write::Encoder<'static, AgeWriter>;
// 第一层：Tar 归档构建器 (最外层接口)
type TarBuilder = tar::Builder<ZstdWriter>;

pub struct StreamBatcher {
    // 管道是 Option 的，因为 finish() 会消耗掉它
    pipeline: Option<TarBuilder>,

    // 状态追踪
    start_time: Option<DateTime<Utc>>,
    item_count: usize,

    // 加密公钥
    recipient: age::x25519::Recipient,
}

impl StreamBatcher {
    pub fn new(public_key: &str) -> Self {
        let recipient = public_key.parse().expect("Invalid Age public key");
        Self {
            pipeline: None,
            start_time: None,
            item_count: 0,
            recipient,
        }
    }

    // 初始化管道 (懒加载)
    fn init_pipeline(&mut self) -> Result<(), Error> {
        if self.pipeline.is_some() {
            return Ok(());
        }

        // 1. 准备接收用的内存 Buffer (预分配 2MB)
        let buffer: MemBuffer = Vec::with_capacity(2 * 1024 * 1024);

        // 2. 构建 Age 加密层 (Layer 3)
        let encryptor =
            age::Encryptor::with_recipients(iter::once(&self.recipient as &dyn age::Recipient))
                .context("Failed to create age encryptor with provided recipients")?;

        let age_writer = encryptor
            .wrap_output(buffer)
            .context("Failed to wrap age output")?;

        // 3. 构建 Zstd 压缩层 (Layer 2)
        // Level 3 是速度和压缩率的最佳平衡
        let zstd_writer = zstd::stream::write::Encoder::new(age_writer, 3)
            .context("Failed to initialize zstd compression layer")?;

        // 4. 构建 Tar 归档层 (Layer 1)
        let tar_builder = tar::Builder::new(zstd_writer);

        self.pipeline = Some(tar_builder);
        self.start_time = Some(Utc::now());
        self.item_count = 0;

        info!("✨ 新的批处理管道已建立 (Tar -> Zstd -> Age)");
        Ok(())
    }

    // 核心写入方法：处理单张图片
    // 注意：这个方法包含 JPEG 编码和加密计算，必须在 spawn_blocking 中运行
    pub fn append(&mut self, event: &MonitorImageEvent) -> Result<()> {
        // 确保管道存在
        self.init_pipeline();

        let filename = event.filename()?;

        // A. 图片转码 (CPU 密集)
        // 我们直接将 JPEG 写入一个临时的小 Buffer，而不是直接喂给 tar
        // 这样可以精确获取文件大小用于 Tar Header
        let mut jpeg_buffer = Vec::new();
        event.image.write_to(
            &mut std::io::Cursor::new(&mut jpeg_buffer),
            ImageFormat::Jpeg,
        )?;

        // B. 构建 Tar Header
        let mut header = tar::Header::new_gnu();
        header.set_size(jpeg_buffer.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();

        // C. 写入管道 (触发 流式压缩 + 流式加密)
        if let Some(builder) = self.pipeline.as_mut() {
            builder.append_data(&mut header, &filename, &mut jpeg_buffer.as_slice())?;
            // 可选：builder.get_mut().flush()?; // 确保数据推入 Age 层
            self.item_count += 1;
        }

        Ok(())
    }
}
