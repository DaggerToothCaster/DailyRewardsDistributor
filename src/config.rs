use anyhow::{anyhow, Result};
use ethers::types::{Address, U256};
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub rpc_url: String,
    pub private_key: String,
    pub contract_address: Address,
    pub chain_id: u64,
    pub gas_limit: U256,
    pub gas_price: Option<U256>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let rpc_url = env::var("RPC_URL")
            .map_err(|_| anyhow!("RPC_URL 环境变量未设置"))?;
        
        let private_key = env::var("PRIVATE_KEY")
            .map_err(|_| anyhow!("PRIVATE_KEY 环境变量未设置"))?;
        
        let contract_address = env::var("CONTRACT_ADDRESS")
            .map_err(|_| anyhow!("CONTRACT_ADDRESS 环境变量未设置"))?
            .parse::<Address>()
            .map_err(|_| anyhow!("无效的合约地址格式"))?;
        
        let chain_id = env::var("CHAIN_ID")
            .unwrap_or_else(|_| "1".to_string())
            .parse::<u64>()
            .map_err(|_| anyhow!("无效的链ID格式"))?;
        
        let gas_limit = env::var("GAS_LIMIT")
            .unwrap_or_else(|_| "500000".to_string())
            .parse::<U256>()
            .map_err(|_| anyhow!("无效的Gas限制格式"))?;
        
        let gas_price = env::var("GAS_PRICE")
            .ok()
            .map(|price| price.parse::<U256>())
            .transpose()
            .map_err(|_| anyhow!("无效的Gas价格格式"))?;
        
        Ok(Config {
            rpc_url,
            private_key,
            contract_address,
            chain_id,
            gas_limit,
            gas_price,
        })
    }
}