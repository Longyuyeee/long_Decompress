use long_compress_assistant::services::universal_engine::UniversalCliEngine;
use long_compress_assistant::services::archive_engine::ArchiveEngine;
use long_compress_assistant::models::compression::TaskLogSeverity;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::path::Path;

#[tokio::main]
async fn main() {
    println!("=== 开始通用引擎 (UniversalEngine) 深度验证 ===");

    // 1. 验证解析逻辑 (由于 parse_progress 是私有的，我们在单元测试中验证，Example中验证公开接口)
    let engine = UniversalCliEngine::new();
    println!("引擎名称: {}", engine.name());

    // 2. 验证环境探测
    let has_7z = std::process::Command::new("7z").arg("--help").output().is_ok() ||
                 std::process::Command::new("7za").arg("--help").output().is_ok();
    
    println!("本地环境 7z 检测结果: {}", if has_7z { "已安装" } else { "未安装 (测试将受限)" });

    // 3. 验证 Trait 接口连通性
    // 我们定义一个通用的解压任务尝试
    let on_progress = Arc::new(|p: f32| {
        println!("  [进度回调] {:.1}%", p * 100.0);
    });

    let on_log = Arc::new(|msg: String, sev: long_compress_assistant::models::compression::TaskLogSeverity| {
        println!("  [日志回调] {:?}: {}", sev, msg);
    });

    let is_cancelled = Arc::new(AtomicBool::new(false));

    // 尝试一个不存在的文件，验证错误处理
    let result = engine.extract_with_progress(
        Path::new("non_existent_file.zip"),
        Path::new("./output"),
        None,
        on_progress,
        on_log,
        is_cancelled
    ).await;

    match result {
        Ok(_) => println!("意外：解压不存在的文件居然成功了？"),
        Err(e) => println!("预期的错误捕获 (文件不存在): {}", e),
    }

    println!("=== 通用引擎验证脚本执行结束 ===");
}
