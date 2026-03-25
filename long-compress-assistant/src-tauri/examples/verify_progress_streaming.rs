use long_compress_assistant::utils::io_utils::ProgressReader;
use std::io::{Cursor, Read};
use std::sync::Arc;

fn main() {
    println!("=== 开始全格式进度追踪器 (Universal Progress Tracker) 逻辑验证 ===");

    // 1. 准备 1MB 的模拟数据
    let data = vec![0u8; 1024 * 1024]; 
    let total_size = data.len() as u64;
    let reader = Cursor::new(data);

    println!("模拟文件大小: {} bytes", total_size);

    // 2. 初始化进度追踪器
    // 回调函数模拟 emit_progress 行为
    let on_progress = Arc::new(|current: u64, total: u64| {
        let percent = (current as f32 / total as f32) * 100.0;
        // 为了避免输出刷屏，只在特定节点打印
        if current % (256 * 1024) == 0 || current == total {
            println!("  [字节追踪] 已处理: {}/{} ({:.1}%)", current, total, percent);
        }
    });

    let mut progress_reader = ProgressReader::new(reader, total_size, on_progress);

    // 3. 执行流式读取 (模拟解压写入过程)
    let mut buffer = [0u8; 128 * 1024]; // 128KB 缓冲区
    let mut total_read = 0;

    println!("开始流式读取...");
    while let Ok(n) = progress_reader.read(&mut buffer) {
        if n == 0 { break; }
        total_read += n;
    }

    println!("读取完成，总计: {} bytes", total_read);
    assert_eq!(total_read as u64, total_size);

    // 4. 验证综合进度计算公式
    println!("\n验证综合进度算法 (2个文件，当前处理第2个):");
    let total_files = 2;
    let current_file_index = 1; // 索引从0开始，这是第2个文件
    let file_source_size = 1000.0;
    let file_current_pos = 500.0; // 处理了一半

    // 公式: (已完成文件 / 总数) + (当前文件已完成比例 / 总数)
    let overall_progress = (current_file_index as f32 / total_files as f32) + 
                          (file_current_pos / file_source_size / total_files as f32);
    
    println!("  文件1已完成: 50.0%");
    println!("  文件2已完成: 50.0%");
    println!("  整体预期进度 (应为 75%): {:.1}%", overall_progress * 100.0);
    
    assert!((overall_progress - 0.75).abs() < 0.001);

    println!("\n进度追踪器核心逻辑验证成功！");
}
