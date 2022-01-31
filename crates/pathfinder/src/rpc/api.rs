//! Implementation of JSON-RPC endpoints.
use crate::{
    core::{
        CallResultValue, ContractAddress, StarknetChainId, StarknetProtocolVersion,
        StarknetTransactionHash, StarknetTransactionIndex, StorageAddress, StorageValue,
    },
    rpc::types::{
        reply::{Block, Code, ErrorCode, StateUpdate, Syncing, Transaction, TransactionReceipt},
        request::{BlockResponseScope, Call},
        BlockHashOrTag, BlockNumberOrTag, Tag,
    },
    sequencer::{error::StarknetError, reply as raw, Client},
};
use core::num;
use jsonrpsee::types::{
    error::{CallError, Error},
    RpcResult,
};
use std::convert::TryInto;

/// Helper function.
fn transaction_index_not_found(index: usize) -> Error {
    Error::Call(CallError::InvalidParams(anyhow::anyhow!(
        "transaction index {} not found",
        index
    )))
}

/// Implements JSON-RPC endpoints.
///
/// __TODO__ directly calls [sequencer::Client](crate::sequencer::Client) until storage is implemented.
pub struct RpcApi(Client);

impl Default for RpcApi {
    fn default() -> Self {
        let module = Client::goerli().expect("failed to initialize sequencer client");
        Self(module)
    }
}

/// Based on [the Starknet operator API spec](https://github.com/starkware-libs/starknet-adrs/blob/master/api/starknet_operator_api_openrpc.json).
impl RpcApi {
    /// Helper function.
    async fn get_raw_block_by_hash(&self, block_hash: BlockHashOrTag) -> RpcResult<raw::Block> {
        // TODO get this from storage
        let block = self.0.block_by_hash(block_hash).await?;
        Ok(block)
    }

    /// Get block information given the block hash.
    /// `block_hash` is the [Hash](crate::rpc::types::BlockHashOrTag::Hash) or [Tag](crate::rpc::types::BlockHashOrTag::Tag)
    /// of the requested block.
    pub async fn get_block_by_hash(
        &self,
        block_hash: BlockHashOrTag,
        requested_scope: Option<BlockResponseScope>,
    ) -> RpcResult<Block> {
        let block = self.get_raw_block_by_hash(block_hash).await?;
        let scope = requested_scope.unwrap_or_default();
        Ok(Block::from_scoped(block, scope))
    }

    /// Helper function.
    async fn get_raw_block_by_number(
        &self,
        block_number: BlockNumberOrTag,
    ) -> RpcResult<raw::Block> {
        let block = self.0.block_by_number(block_number).await?;
        Ok(block)
    }

    /// Get block information given the block number (its height).
    /// `block_number` is the [Number](crate::rpc::types::BlockNumberOrTag::Number) (height) or [Tag](crate::rpc::types::BlockHashOrTag::Tag)
    /// of the requested block.
    pub async fn get_block_by_number(
        &self,
        block_number: BlockNumberOrTag,
        requested_scope: Option<BlockResponseScope>,
    ) -> RpcResult<Block> {
        let block = self.get_raw_block_by_number(block_number).await?;
        let scope = requested_scope.unwrap_or_default();
        Ok(Block::from_scoped(block, scope))
    }

    /// Get the information about the result of executing the requested block.
    /// `block_hash` is the [Hash](crate::rpc::types::BlockHashOrTag::Hash) or [Tag](crate::rpc::types::BlockHashOrTag::Tag)
    /// of the requested block.
    pub async fn get_state_update_by_hash(
        &self,
        block_hash: BlockHashOrTag,
    ) -> RpcResult<StateUpdate> {
        // TODO get this from storage or directly from L1
        match block_hash {
            BlockHashOrTag::Tag(Tag::Latest) => todo!("Implement L1 state diff retrieval."),
            BlockHashOrTag::Tag(Tag::Pending) => {
                todo!("Implement when sequencer support for pending tag available.")
            }
            BlockHashOrTag::Hash(_) => todo!("Implement L1 state diff retrieval."),
        }
    }

    /// Get the value of the storage at the given address and key.
    /// `contract_address` is the address of the contract to read from, `key` is the key to the storage value for the given contract,
    /// `block_hash` is the [Hash](crate::rpc::types::BlockHashOrTag::Hash) or [Tag](crate::rpc::types::BlockHashOrTag::Tag)
    /// of the requested block.
    pub async fn get_storage_at(
        &self,
        contract_address: ContractAddress,
        key: StorageAddress,
        block_hash: BlockHashOrTag,
    ) -> RpcResult<StorageValue> {
        let storage_val = self.0.storage(contract_address, key, block_hash).await?;
        Ok(storage_val)
    }

    /// Helper function.
    async fn get_raw_transaction_by_hash(
        &self,
        transaction_hash: StarknetTransactionHash,
    ) -> RpcResult<raw::Transaction> {
        // TODO get this from storage
        let txn = self.0.transaction(transaction_hash).await?;
        if txn.status == raw::Status::NotReceived {
            return Err(ErrorCode::InvalidTransactionHash.into());
        }
        Ok(txn)
    }

    /// Get the details and status of a submitted transaction.
    /// `transaction_hash` is the hash of the requested transaction.
    pub async fn get_transaction_by_hash(
        &self,
        transaction_hash: StarknetTransactionHash,
    ) -> RpcResult<Transaction> {
        // TODO get this from storage
        let txn = self.get_raw_transaction_by_hash(transaction_hash).await?;
        Ok(txn.into())
    }

    /// Get the details of a transaction by a given block hash and index.
    /// `block_hash` is the [Hash](crate::rpc::types::BlockHashOrTag::Hash) or [Tag](crate::rpc::types::BlockHashOrTag::Tag)
    /// of the requested block.
    pub async fn get_transaction_by_block_hash_and_index(
        &self,
        block_hash: BlockHashOrTag,
        index: StarknetTransactionIndex,
    ) -> RpcResult<Transaction> {
        // TODO get this from storage
        let block = self.get_raw_block_by_hash(block_hash).await?;
        let index: usize = index
            .0
            .try_into()
            .map_err(|e| Error::Call(CallError::InvalidParams(anyhow::Error::new(e))))?;

        block.transactions.into_iter().nth(index).map_or(
            Err(transaction_index_not_found(index)),
            |txn| Ok(txn.into()),
        )
    }

    /// Get the details of a transaction by a given block number and index.
    /// `block_number` is the [Number](crate::rpc::types::BlockNumberOrTag::Number) (height) or [Tag](crate::rpc::types::BlockHashOrTag::Tag)
    /// of the requested block.
    pub async fn get_transaction_by_block_number_and_index(
        &self,
        block_number: BlockNumberOrTag,
        index: StarknetTransactionIndex,
    ) -> RpcResult<Transaction> {
        // TODO get this from storage
        let block = self.get_raw_block_by_number(block_number).await?;
        let index: usize = index
            .0
            .try_into()
            .map_err(|e| Error::Call(CallError::InvalidParams(anyhow::Error::new(e))))?;

        block.transactions.into_iter().nth(index).map_or(
            Err(transaction_index_not_found(index)),
            |txn| Ok(txn.into()),
        )
    }

    /// Get the transaction receipt by the transaction hash.
    /// `transaction_hash` is the hash of the requested transaction.
    pub async fn get_transaction_receipt(
        &self,
        transaction_hash: StarknetTransactionHash,
    ) -> RpcResult<TransactionReceipt> {
        let txn = self.get_raw_transaction_by_hash(transaction_hash).await?;
        if let Some(block_hash) = txn.block_hash {
            if let Some(index) = txn.transaction_index {
                let block = self
                    .get_raw_block_by_hash(BlockHashOrTag::Hash(block_hash))
                    .await?;
                let index: usize = index
                    .try_into()
                    .map_err(|e| Error::Call(CallError::InvalidParams(anyhow::Error::new(e))))?;
                block
                    .transaction_receipts
                    .into_iter()
                    .nth(index)
                    .map_or(Err(transaction_index_not_found(index)), |receipt| {
                        Ok(TransactionReceipt::with_status(receipt, block.status))
                    })
            } else {
                Err(Error::Call(CallError::InvalidParams(anyhow::anyhow!(
                    "transaction index not found"
                ))))
            }
        } else {
            Err(ErrorCode::InvalidBlockHash.into())
        }
    }

    /// Get the code of a specific contract.
    /// `contract_address` is the address of the contract to read from.
    pub async fn get_code(&self, contract_address: ContractAddress) -> RpcResult<Code> {
        let code = self
            .0
            .code(contract_address, BlockHashOrTag::Tag(Tag::Latest))
            .await?;
        Ok(code)
    }

    /// Get the number of transactions in a block given a block hash.
    /// `block_hash` is the [Hash](crate::rpc::types::BlockHashOrTag::Hash) or [Tag](crate::rpc::types::BlockHashOrTag::Tag)
    /// of the requested block.
    pub async fn get_block_transaction_count_by_hash(
        &self,
        block_hash: BlockHashOrTag,
    ) -> RpcResult<u64> {
        // TODO get this from storage
        let block = self.get_raw_block_by_hash(block_hash).await?;
        let len: u64 = block
            .transactions
            .len()
            .try_into()
            .map_err(|e| Error::Call(CallError::InvalidParams(anyhow::Error::new(e))))?;
        Ok(len)
    }

    /// Get the number of transactions in a block given a block hash.
    /// `block_hash` is the [Hash](crate::rpc::types::BlockHashOrTag::Hash) or [Tag](crate::rpc::types::BlockHashOrTag::Tag)
    /// of the requested block.
    pub async fn get_block_transaction_count_by_number(
        &self,
        block_number: BlockNumberOrTag,
    ) -> RpcResult<u64> {
        // TODO get this from storage
        let block = self.get_raw_block_by_number(block_number).await?;
        let len: u64 = block
            .transactions
            .len()
            .try_into()
            .map_err(|e| Error::Call(CallError::InvalidParams(anyhow::Error::new(e))))?;
        Ok(len)
    }

    /// Call a starknet function without creating a StarkNet transaction.
    /// `block_hash` is the [Hash](crate::rpc::types::BlockHashOrTag::Hash) or [Tag](crate::rpc::types::BlockHashOrTag::Tag)
    /// of the requested block.
    pub async fn call(
        &self,
        request: Call,
        block_hash: BlockHashOrTag,
    ) -> RpcResult<Vec<CallResultValue>> {
        let call = self.0.call(request.into(), block_hash).await?;
        Ok(call.result)
    }

    /// Get the most recent accepted block number.
    pub async fn block_number(&self) -> RpcResult<u64> {
        let block = self
            .0
            .block_by_hash(BlockHashOrTag::Tag(Tag::Latest))
            .await?;
        let number = block.block_number.ok_or(anyhow::anyhow!(
            "Block number field missing in latest block."
        ))?;
        Ok(number.0)
    }

    /// Return the currently configured StarkNet chain id.
    pub async fn chain_id(&self) -> RpcResult<StarknetChainId> {
        todo!("Figure out where to take it from.")
    }

    /// Returns the transactions in the transaction pool, recognized by this sequencer.
    pub async fn pending_transactions(&self) -> RpcResult<Vec<Transaction>> {
        todo!("Figure out where to take them from.")
    }

    /// Returns the current starknet protocol version identifier, as supported by this node.
    pub async fn protocol_version(&self) -> RpcResult<StarknetProtocolVersion> {
        todo!("Figure out where to take it from.")
    }

    /// Returns an object about the sync status, or false if the node is not synching.
    pub async fn syncing(&self) -> RpcResult<Syncing> {
        todo!("Figure out where to take it from.")
    }
}
