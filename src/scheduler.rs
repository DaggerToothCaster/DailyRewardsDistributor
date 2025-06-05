use anyhow::Result;
use chrono::{DateTime, Local, Timelike};
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{info, warn};

pub struct DailyScheduler {
    scheduler: JobScheduler,
}

impl DailyScheduler {
    pub async fn new() -> Result<Self> {
        let scheduler = JobScheduler::new().await?;
        Ok(Self { scheduler })
    }
    
    pub async fn add_daily_job<F, Fut>(&self, task: F) -> Result<()>
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        // 每天0点执行的Cron表达式
        let job = Job::new_async("0 0 14 * * *", {
            let task = std::sync::Arc::new(task);
            move |_uuid, _l| {
            let task = task.clone();
            Box::pin(async move {
                info!("开始执行每日任务...");
                let now = Local::now();
                info!("当前时间: {}", now.format("%Y-%m-%d %H:%M:%S"));
                
                match (task)().await {
                Ok(_) => info!("每日任务执行成功"),
                Err(e) => warn!("每日任务执行失败: {}", e),
                }
            })
            }
        })?;
        
        self.scheduler.add(job).await?;
        info!("每日任务已添加到调度器");
        Ok(())
    }
    
    pub async fn add_test_job<F, Fut>(&self, task: F) -> Result<()>
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        // 每分钟执行一次的测试任务
        let job = Job::new_async("0 * * * * *", {
            let task = std::sync::Arc::new(task);
            move |_uuid, _l| {
                let task = task.clone();
                Box::pin(async move {
                    info!("执行测试任务...");
                    match (task)().await {
                        Ok(_) => info!("测试任务执行成功"),
                        Err(e) => warn!("测试任务执行失败: {}", e),
                    }
                })
            }
        })?;
        
        self.scheduler.add(job).await?;
        info!("测试任务已添加到调度器（每分钟执行）");
        Ok(())
    }
    
    pub async fn start(&self) -> Result<()> {
        self.scheduler.start().await?;
        info!("调度器已启动");
        Ok(())
    }
    
    pub async fn shutdown(&mut self) -> Result<()> {
        self.scheduler.shutdown().await?;
        info!("调度器已关闭");
        Ok(())
    }
    
    pub fn next_midnight() -> DateTime<Local> {
        let now = Local::now();
        let tomorrow = now.date_naive().succ_opt().unwrap();
        tomorrow.and_hms_opt(0, 0, 0).unwrap().and_local_timezone(Local).unwrap()
    }
}