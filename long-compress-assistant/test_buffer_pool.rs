use long_compress_assistant::services::io_buffer_pool_benchmark::run_complete_benchmark_suite;
use tokio::runtime::Runtime;

fn main() {
    // 创建Tokio运行时
    let rt = Runtime::new().unwrap();

    // 运行基准测试套件
    let result = rt.block_on(async {
        run_complete_benchmark_suite().await
    });

    // 输出结果
    println!("{}", result);
    println!("\n=== 缓冲区池优化测试完成 ===");

    // 分析结果
    analyze_results(&result);
}

fn analyze_results(results: &str) {
    println!("\n=== 性能优化分析 ===");

    let lines: Vec<&str> = results.lines().collect();
    let mut current_section = "";

    for line in lines {
        if line.starts_with("===") {
            current_section = line;
        } else if line.contains("吞吐量") {
            println!("{}", current_section);
            println!("{}", line);

            // 提取吞吐量数值
            if let Some(start) = line.find(": ") {
                if let Some(end) = line.find(" 操作/秒") {
                    let throughput_str = &line[start+2..end];
                    if let Ok(throughput) = throughput_str.parse::<f64>() {
                        if throughput > 1000.0 {
                            println!("✅ 吞吐量优秀: {:.0} 操作/秒", throughput);
                        } else if throughput > 100.0 {
                            println!("⚠️  吞吐量一般: {:.0} 操作/秒", throughput);
                        } else {
                            println!("❌ 吞吐量较低: {:.0} 操作/秒", throughput);
                        }
                    }
                }
            }
        } else if line.contains("加速比") {
            println!("{}", current_section);
            println!("{}", line);

            // 提取加速比数值
            if let Some(start) = line.find(": ") {
                if let Some(end) = line.find("x") {
                    let speedup_str = &line[start+2..end];
                    if let Ok(speedup) = speedup_str.parse::<f64>() {
                        if speedup > 2.0 {
                            println!("✅ 显著性能提升: {:.1}x 加速", speedup);
                        } else if speedup > 1.2 {
                            println!("⚠️  中等性能提升: {:.1}x 加速", speedup);
                        } else {
                            println!("❌ 性能提升不明显: {:.1}x 加速", speedup);
                        }
                    }
                }
            }
        } else if line.contains("内存节省") {
            println!("{}", current_section);
            println!("{}", line);

            // 提取内存节省百分比
            if let Some(start) = line.find(": ") {
                if let Some(end) = line.find("%") {
                    let saving_str = &line[start+2..end];
                    if let Ok(saving) = saving_str.parse::<f64>() {
                        if saving > 50.0 {
                            println!("✅ 内存节省显著: {:.1}%", saving);
                        } else if saving > 20.0 {
                            println!("⚠️  内存节省一般: {:.1}%", saving);
                        } else {
                            println!("❌ 内存节省不明显: {:.1}%", saving);
                        }
                    }
                }
            }
        } else if line.contains("缓冲区命中率") {
            println!("{}", current_section);
            println!("{}", line);

            // 提取命中率
            if let Some(start) = line.find(": ") {
                if let Some(end) = line.find("%") {
                    let hit_rate_str = &line[start+2..end];
                    if let Ok(hit_rate) = hit_rate_str.parse::<f64>() {
                        if hit_rate > 80.0 {
                            println!("✅ 缓冲区命中率优秀: {:.1}%", hit_rate);
                        } else if hit_rate > 60.0 {
                            println!("⚠️  缓冲区命中率一般: {:.1}%", hit_rate);
                        } else {
                            println!("❌ 缓冲区命中率较低: {:.1}%", hit_rate);
                        }
                    }
                }
            }
        }
    }

    println!("\n=== 优化建议 ===");
    println!("1. 如果吞吐量较低，考虑增加缓冲区池的初始大小");
    println!("2. 如果内存节省不明显，检查缓冲区重用策略");
    println!("3. 如果命中率低，调整缓冲区大小范围");
    println!("4. 考虑根据实际工作负载调整并发参数");
}