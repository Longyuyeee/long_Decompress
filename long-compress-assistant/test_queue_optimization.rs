use long_compress_assistant::task_queue::queue_benchmark::QueueBenchmark;
use tokio::runtime::Runtime;

fn main() {
    println!("=== 任务队列锁优化性能测试 ===\n");

    // 创建Tokio运行时
    let rt = Runtime::new().unwrap();

    // 创建基准测试
    let benchmark = QueueBenchmark::new(10000, 8);

    // 运行完整的性能测试套件
    let result = rt.block_on(async {
        benchmark.run_complete_benchmark_suite().await
    });

    // 输出结果
    println!("{}", result.results);
    println!("\n=== 任务队列锁优化测试完成 ===");

    // 分析结果
    analyze_results(&result.results);
}

fn analyze_results(results: &str) {
    println!("\n=== 性能优化分析 ===");

    let lines: Vec<&str> = results.lines().collect();
    let mut current_section = "";
    let mut has_optimization = false;

    for line in lines {
        if line.starts_with("===") {
            current_section = line;
        } else if line.contains("加速比") {
            println!("{}", current_section);
            println!("{}", line);

            // 提取加速比数值
            if let Some(start) = line.find(": ") {
                if let Some(end) = line.find("x") {
                    let speedup_str = &line[start+2..end];
                    if let Ok(speedup) = speedup_str.parse::<f64>() {
                        if speedup > 3.0 {
                            println!("✅ 性能优化显著: {:.1}x 加速", speedup);
                            has_optimization = true;
                        } else if speedup > 1.5 {
                            println!("⚠️  性能优化中等: {:.1}x 加速", speedup);
                            has_optimization = true;
                        } else if speedup > 1.1 {
                            println!("❌ 性能优化轻微: {:.1}x 加速", speedup);
                        } else {
                            println!("❌ 无性能优化: {:.1}x 加速", speedup);
                        }
                    }
                }
            }
        } else if line.contains("吞吐量") && line.contains("优化队列") {
            println!("{}", current_section);
            println!("{}", line);

            // 提取吞吐量数值
            if let Some(start) = line.find(": ") {
                if let Some(end) = line.find(" 任务/秒") {
                    let throughput_str = &line[start+2..end];
                    if let Ok(throughput) = throughput_str.parse::<f64>() {
                        if throughput > 10000.0 {
                            println!("✅ 吞吐量优秀: {:.0} 任务/秒", throughput);
                        } else if throughput > 1000.0 {
                            println!("⚠️  吞吐量良好: {:.0} 任务/秒", throughput);
                        } else {
                            println!("❌ 吞吐量较低: {:.0} 任务/秒", throughput);
                        }
                    }
                }
            }
        } else if line.contains("负载均衡改善") {
            println!("{}", current_section);
            println!("{}", line);

            // 提取负载均衡改善值
            if let Some(start) = line.find(": ") {
                let balance_str = &line[start+2..];
                if let Ok(balance_improvement) = balance_str.parse::<f64>() {
                    if balance_improvement > 0.3 {
                        println!("✅ 负载均衡显著改善: {:.3}", balance_improvement);
                        has_optimization = true;
                    } else if balance_improvement > 0.1 {
                        println!("⚠️  负载均衡中等改善: {:.3}", balance_improvement);
                        has_optimization = true;
                    } else if balance_improvement > 0.0 {
                        println!("❌ 负载均衡轻微改善: {:.3}", balance_improvement);
                    } else {
                        println!("❌ 负载均衡无改善: {:.3}", balance_improvement);
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
                        if saving > 30.0 {
                            println!("✅ 内存节省显著: {:.1}%", saving);
                            has_optimization = true;
                        } else if saving > 10.0 {
                            println!("⚠️  内存节省一般: {:.1}%", saving);
                            has_optimization = true;
                        } else if saving > 0.0 {
                            println!("❌ 内存节省轻微: {:.1}%", saving);
                        } else {
                            println!("❌ 无内存节省: {:.1}%", saving);
                        }
                    }
                }
            }
        } else if line.contains("效率") && line.contains("优化队列") {
            println!("{}", current_section);
            println!("{}", line);

            // 提取效率百分比
            if let Some(start) = line.find(": ") {
                if let Some(end) = line.find("%") {
                    let efficiency_str = &line[start+2..end];
                    if let Ok(efficiency) = efficiency_str.parse::<f64>() {
                        if efficiency > 95.0 {
                            println!("✅ 任务处理效率优秀: {:.1}%", efficiency);
                        } else if efficiency > 80.0 {
                            println!("⚠️  任务处理效率良好: {:.1}%", efficiency);
                        } else {
                            println!("❌ 任务处理效率较低: {:.1}%", efficiency);
                        }
                    }
                }
            }
        }
    }

    println!("\n=== 优化总结 ===");
    if has_optimization {
        println!("✅ 任务队列锁优化实施成功！");
        println!("优化措施包括：");
        println!("1. 使用无锁队列减少锁竞争");
        println!("2. 实现工作窃取提高并发性能");
        println!("3. 优化锁粒度提升吞吐量");
        println!("4. 改进内存使用效率");
    } else {
        println!("❌ 任务队列锁优化效果不明显");
        println!("建议进一步优化：");
        println!("1. 分析锁竞争热点");
        println!("2. 调整工作窃取参数");
        println!("3. 优化任务调度算法");
        println!("4. 考虑使用更高效的数据结构");
    }

    println!("\n=== 后续优化建议 ===");
    println!("1. 根据实际工作负载调整队列参数");
    println!("2. 实现动态工作窃取策略");
    println!("3. 添加队列监控和自动调优");
    println!("4. 考虑使用更高级的并发原语");
    println!("5. 实施A/B测试验证优化效果");
}