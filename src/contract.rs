use anyhow::Result;
use ethers::prelude::*;
use ethers::types::transaction::eip2718::TypedTransaction;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

// 定义合约ABI - 添加更多调试函数
abigen!(
    RewardsContractABI,
    r#"[
        function distributeDailyRewards() external
        function isActive() external view returns (bool)
        function owner() external view returns (address)
        function lastDistributionTime() external view returns (uint256)
        function paused() external view returns (bool)
        function canDistribute() external view returns (bool)
        function getRewardsPool() external view returns (uint256)
        function totalRewards() external view returns (uint256)
        function rewardsPerDay() external view returns (uint256)
    ]"#
);

#[derive(Clone)]
pub struct RewardsContract {
    contract: RewardsContractABI<SignerMiddleware<Provider<Http>, LocalWallet>>,
    pub client: Arc<SignerMiddleware<Provider<Http>, LocalWallet>>,
    gas_limit: U256,
    gas_price: Option<U256>,
    chain_id: u64,
}

impl RewardsContract {
    pub fn new(
        address: Address,
        client: Arc<SignerMiddleware<Provider<Http>, LocalWallet>>,
        gas_limit: U256,
        gas_price: Option<U256>,
        chain_id: u64,
    ) -> Self {
        let contract = RewardsContractABI::new(address, client.clone());

        Self {
            contract,
            client,
            gas_limit,
            gas_price,
            chain_id,
        }
    }

    pub async fn distribute_daily_rewards(&self) -> Result<H256> {
        info!("准备分发每日奖励...");

        // 1. 执行预检查
        self.pre_distribution_checks().await?;

        // 2. 模拟交易执行
        if let Err(e) = self.simulate_distribution().await {
            error!("交易模拟失败: {}", e);
            return Err(e);
        }

        info!("预检查和模拟都通过，开始发送交易...");

        // 3. 估算Gas
        let gas_estimate = match self.estimate_gas_safely().await {
            Ok(gas) => {
                info!("Gas估算成功: {}", gas);
                gas
            }
            Err(e) => {
                warn!("Gas估算失败，使用默认值: {}", e);
                self.gas_limit
            }
        };

        let gas_with_buffer = if self.is_local_development_network() {
            gas_estimate * 150 / 100
        } else {
            gas_estimate * 120 / 100
        };

        info!("使用Gas限制: {}", gas_with_buffer);

        // 4. 构建并发送交易
        let tx_request = self.build_legacy_transaction(gas_with_buffer).await?;

        info!("发送交易到网络...");
        let pending_tx = self.client.send_transaction(tx_request, None).await?;
        let tx_hash = pending_tx.tx_hash();

        info!("交易已发送，哈希: {:?}", tx_hash);

        Ok(tx_hash)
    }

    /// 执行分发前的所有检查
    async fn pre_distribution_checks(&self) -> Result<()> {
        info!("执行分发前检查...");

        // 检查合约基本状态
        let contract_status = self.get_contract_status().await?;
        info!("合约状态检查结果: {:#?}", contract_status);

        // 检查调用者权限
        let caller = self.client.address();
      
        info!("调用者 {:?} ", caller);

        // 检查合约是否活跃
        // if !contract_status.is_active {
        //     return Err(anyhow::anyhow!("合约当前不活跃"));
        // }

        // // 检查是否暂停
        // if contract_status.is_paused {
        //     return Err(anyhow::anyhow!("合约当前已暂停"));
        // }

        // // 检查是否可以分发
        // if !contract_status.can_distribute {
        //     return Err(anyhow::anyhow!("当前不允许分发奖励（可能时间间隔不够）"));
        // }

        // // 检查奖励池余额
        // if contract_status.rewards_pool == U256::zero() {
        //     return Err(anyhow::anyhow!("奖励池余额为零"));
        // }

        // 检查时间间隔
        // if let Some(last_time) = contract_status.last_distribution_time {
        //     let current_time = std::time::SystemTime::now()
        //         .duration_since(std::time::UNIX_EPOCH)?
        //         .as_secs();

        //     let time_diff = current_time - last_time.as_u64();
        //     const MIN_INTERVAL: u64 = 23 * 3600; // 23小时

        //     if time_diff < MIN_INTERVAL && !self.is_local_development_network() {
        //         let hours_remaining = (MIN_INTERVAL - time_diff) / 3600;
        //         return Err(anyhow::anyhow!(
        //             "距离上次分发时间不足23小时，还需等待 {} 小时",
        //             hours_remaining
        //         ));
        //     }
        // }

        info!("所有预检查都通过");
        Ok(())
    }

    /// 模拟交易执行
    async fn simulate_distribution(&self) -> Result<()> {
        info!("模拟交易执行...");

        // 使用eth_call模拟交易
        let call_data = self
            .contract
            .distribute_daily_rewards()
            .calldata()
            .ok_or_else(|| anyhow::anyhow!("无法生成调用数据"))?;

        let tx_request = TransactionRequest {
            to: Some(self.contract.address().into()),
            data: Some(call_data),
            from: Some(self.client.address()),
            gas: Some(self.gas_limit),
            gas_price: self.gas_price,
            ..Default::default()
        };

        let typed_tx: TypedTransaction = tx_request.into();
        match self.client.call(&typed_tx, None).await {
            Ok(_) => {
                info!("交易模拟成功");
                Ok(())
            }
            Err(e) => {
                error!("交易模拟失败: {}", e);

                // 尝试解析具体的revert原因
                if let Some(revert_reason) = self.parse_revert_reason(&e).await {
                    error!("Revert原因: {}", revert_reason);
                    return Err(anyhow::anyhow!("合约执行被拒绝: {}", revert_reason));
                }

                Err(anyhow::anyhow!("交易模拟失败: {}", e))
            }
        }
    }

    /// 解析revert原因
    async fn parse_revert_reason<E: std::fmt::Debug>(&self, error: &E) -> Option<String> {
        // 尝试从错误信息中提取revert原因
        let error_str = format!("{:?}", error);

        // 常见的revert原因模式
        if error_str.contains("revert") {
            if error_str.contains("Ownable: caller is not the owner") {
                return Some("只有合约所有者可以调用此函数".to_string());
            }
            if error_str.contains("Pausable: paused") {
                return Some("合约已暂停".to_string());
            }
            if error_str.contains("Too early") || error_str.contains("time") {
                return Some("时间间隔不够，请稍后再试".to_string());
            }
            if error_str.contains("Insufficient") || error_str.contains("balance") {
                return Some("余额不足".to_string());
            }
            if error_str.contains("Not active") {
                return Some("合约未激活".to_string());
            }
        }

        None
    }

    /// 获取合约状态
    pub async fn get_contract_status(&self) -> Result<ContractStatus> {
        info!("获取合约状态...");

        let mut status = ContractStatus::default();

        // 基本状态检查
        if let Ok(active) = self.contract.is_active().call().await {
            status.is_active = active;
        }

        if let Ok(owner) = self.contract.owner().call().await {
            status.owner = owner;
        }

        if let Ok(last_time) = self.contract.last_distribution_time().call().await {
            status.last_distribution_time = Some(last_time);
        }

        // 可选的状态检查（如果合约不支持这些函数，会被忽略）
        if let Ok(paused) = self.contract.paused().call().await {
            status.is_paused = paused;
        }

        if let Ok(can_distribute) = self.contract.can_distribute().call().await {
            status.can_distribute = can_distribute;
        }

        if let Ok(rewards_pool) = self.contract.get_rewards_pool().call().await {
            status.rewards_pool = rewards_pool;
        }

        if let Ok(total_rewards) = self.contract.total_rewards().call().await {
            status.total_rewards = Some(total_rewards);
        }

        if let Ok(rewards_per_day) = self.contract.rewards_per_day().call().await {
            status.rewards_per_day = Some(rewards_per_day);
        }

        Ok(status)
    }

    /// 构建传统类型的交易
    async fn build_legacy_transaction(&self, gas_limit: U256) -> Result<TransactionRequest> {
        let call_data = self
            .contract
            .distribute_daily_rewards()
            .calldata()
            .ok_or_else(|| anyhow::anyhow!("无法生成调用数据"))?;

        let nonce = self
            .client
            .get_transaction_count(self.client.address(), None)
            .await?;

        let gas_price = self.get_safe_gas_price().await?;

        info!("构建传统交易:");
        info!("  - To: {:?}", self.contract.address());
        info!("  - From: {:?}", self.client.address());
        info!("  - Nonce: {}", nonce);
        info!("  - Gas Limit: {}", gas_limit);
        info!(
            "  - Gas Price: {} Gwei",
            ethers::utils::format_units(gas_price, "gwei").unwrap_or_default()
        );
        info!("  - Data Length: {} bytes", call_data.len());

        let tx_request = TransactionRequest {
            to: Some(self.contract.address().into()),
            value: Some(U256::zero()),
            gas: Some(gas_limit),
            gas_price: Some(gas_price),
            data: Some(call_data),
            nonce: Some(nonce),
            chain_id: Some(self.chain_id.into()),
            ..Default::default()
        };

        Ok(tx_request)
    }

    /// 安全的Gas估算
    async fn estimate_gas_safely(&self) -> Result<U256> {
        // 尝试使用eth_estimateGas
        let call_data = self
            .contract
            .distribute_daily_rewards()
            .calldata()
            .ok_or_else(|| anyhow::anyhow!("无法生成调用数据"))?;

        let tx_request = TransactionRequest {
            to: Some(self.contract.address().into()),
            data: Some(call_data),
            from: Some(self.client.address()),
            ..Default::default()
        };

        let typed_tx: TypedTransaction = tx_request.into();

        match self.client.estimate_gas(&typed_tx, None).await {
            Ok(gas) => Ok(gas),
            Err(e) => {
                warn!("Gas估算失败: {}", e);
                Ok(self.gas_limit)
            }
        }
    }

    /// 获取安全的Gas价格
    async fn get_safe_gas_price(&self) -> Result<U256> {
        if let Some(price) = self.gas_price {
            return Ok(price);
        }

        match self.client.get_gas_price().await {
            Ok(network_price) => {
                let gas_price = if self.is_local_development_network() {
                    let local_gas_price = ethers::utils::parse_units("20", "gwei")?
                        .try_into()
                        .map_err(|_| anyhow::anyhow!("Gas价格转换失败"))?;
                    std::cmp::min(network_price, local_gas_price)
                } else {
                    network_price * 110 / 100
                };
                Ok(gas_price)
            }
            Err(_) => {
                let default_price = if self.is_local_development_network() {
                    ethers::utils::parse_units("20", "gwei")?
                } else {
                    ethers::utils::parse_units("30", "gwei")?
                };

                Ok(default_price
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("默认Gas价格转换失败"))?)
            }
        }
    }

    fn is_local_development_network(&self) -> bool {
        matches!(self.chain_id, 1337 | 31337 | 1338 | 5777)
    }

    pub async fn is_contract_active(&self) -> Result<bool> {
        match self.contract.is_active().call().await {
            Ok(active) => Ok(active),
            Err(e) => {
                warn!("无法检查合约状态: {}", e);
                Ok(true)
            }
        }
    }

    pub async fn get_last_distribution_time(&self) -> Result<U256> {
        let time = self.contract.last_distribution_time().call().await?;
        Ok(time)
    }

    pub async fn wait_for_confirmation(&self, tx_hash: H256) -> Result<TransactionReceipt> {
        info!("等待交易确认: {:?}", tx_hash);

        let timeout = if self.is_local_development_network() {
            std::time::Duration::from_secs(60)
        } else {
            std::time::Duration::from_secs(300)
        };

        let start_time = std::time::Instant::now();

        loop {
            if start_time.elapsed() > timeout {
                return Err(anyhow::anyhow!("交易确认超时"));
            }

            match self.client.get_transaction_receipt(tx_hash).await? {
                Some(receipt) => {
                    if receipt.status == Some(U64::from(1)) {
                        info!("交易执行成功");
                    } else {
                        error!("交易执行失败");
                        // 尝试获取失败原因
                        if let Ok(tx) = self.client.get_transaction(tx_hash).await {
                            if let Some(tx_data) = tx {
                                error!("失败的交易详情: {:?}", tx_data);
                            }
                        }
                    }
                    return Ok(receipt);
                }
                None => {
                    debug!("交易尚未确认，继续等待...");
                    let check_interval = if self.is_local_development_network() {
                        std::time::Duration::from_secs(1)
                    } else {
                        std::time::Duration::from_secs(5)
                    };
                    tokio::time::sleep(check_interval).await;
                }
            }
        }
    }

    pub async fn get_balance(&self) -> Result<U256> {
        let balance = self.client.get_balance(self.client.address(), None).await?;
        Ok(balance)
    }

    pub async fn get_network_info(&self) -> Result<NetworkInfo> {
        let chain_id = self.client.get_chainid().await?;
        let block_number = self.client.get_block_number().await?;
        let gas_price = self.get_safe_gas_price().await.unwrap_or_default();
        let is_local_dev = self.is_local_development_network();

        Ok(NetworkInfo {
            chain_id,
            block_number,
            gas_price,
            is_local_dev,
        })
    }

    pub async fn test_connection(&self) -> Result<()> {
        info!("测试合约连接...");

        let code = self.client.get_code(self.contract.address(), None).await?;
        if code.is_empty() {
            return Err(anyhow::anyhow!("合约地址没有部署代码"));
        }

        info!("合约代码长度: {} bytes", code.len());

        // 获取并显示合约状态
        if let Ok(status) = self.get_contract_status().await {
            info!("合约状态: {:#?}", status);
        }

        Ok(())
    }

    /// 访问器方法 - 用于调试和测试
    pub fn client_address(&self) -> Address {
        self.client.address()
    }

    pub fn contract_address(&self) -> Address {
        self.contract.address()
    }

    pub fn gas_limit(&self) -> U256 {
        self.gas_limit
    }

    pub fn inner_contract(
        &self,
    ) -> &RewardsContractABI<SignerMiddleware<Provider<Http>, LocalWallet>> {
        &self.contract
    }
}

#[derive(Debug, Default)]
pub struct ContractStatus {
    pub is_active: bool,
    pub is_paused: bool,
    pub can_distribute: bool,
    pub owner: Address,
    pub last_distribution_time: Option<U256>,
    pub rewards_pool: U256,
    pub total_rewards: Option<U256>,
    pub rewards_per_day: Option<U256>,
}

#[derive(Debug)]
pub struct NetworkInfo {
    pub chain_id: U256,
    pub block_number: U64,
    pub gas_price: U256,
    pub is_local_dev: bool,
}
