//! 任务队列系统模块
//!
//! 提供任务排队、调度、执行和状态管理功能。

pub mod models;
pub mod task_queue;
pub mod optimized_task_queue;
pub mod queue_benchmark;
pub mod task_scheduler;
pub mod task_executor;
pub mod task_manager;
pub mod task_persistence;
pub mod task_event_log;
pub mod batch_task_processor;

// 重新导出主要类型
pub use models::{
    QueueTask, TaskType, TaskPriority, QueueTaskStatus,
    QueueStatistics, QueueConfig, TaskFilter,
};
pub use task_manager::{TaskManager, GlobalTaskManager, TASK_MANAGER};
pub use batch_task_processor::{
    BatchTaskProcessor, BatchTaskConfig, BatchTaskType, BatchTaskRequest,
    BatchTaskResult, BatchItemResult, BatchTaskProgress, BatchTaskStatus,
};