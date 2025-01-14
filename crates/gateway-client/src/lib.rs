//! Starknet L2 sequencer client.
use pathfinder_common::{BlockHash, BlockId, BlockNumber, ClassHash, StateUpdate, TransactionHash};
use reqwest::Url;
use starknet_gateway_types::reply::PendingBlock;
use starknet_gateway_types::trace::{BlockTrace, TransactionTrace};
use starknet_gateway_types::{error::SequencerError, reply, request};
use std::{fmt::Debug, result::Result, time::Duration};

mod builder;
mod metrics;

#[allow(unused_variables)]
#[mockall::automock]
#[async_trait::async_trait]
pub trait GatewayApi: Sync {
    async fn pending_block(&self) -> Result<(PendingBlock, StateUpdate), SequencerError> {
        unimplemented!();
    }

    async fn block_header(
        &self,
        block: BlockId,
    ) -> Result<(BlockNumber, BlockHash), SequencerError> {
        unimplemented!()
    }

    async fn pending_class_by_hash(
        &self,
        class_hash: ClassHash,
    ) -> Result<bytes::Bytes, SequencerError> {
        unimplemented!();
    }

    async fn pending_casm_by_hash(
        &self,
        class_hash: ClassHash,
    ) -> Result<bytes::Bytes, SequencerError> {
        unimplemented!();
    }

    async fn transaction(
        &self,
        transaction_hash: TransactionHash,
    ) -> Result<reply::TransactionStatus, SequencerError> {
        unimplemented!();
    }

    async fn state_update_with_block(
        &self,
        block: BlockNumber,
    ) -> Result<(reply::Block, StateUpdate), SequencerError> {
        unimplemented!();
    }

    async fn eth_contract_addresses(&self) -> Result<reply::EthContractAddresses, SequencerError> {
        unimplemented!();
    }

    async fn add_invoke_transaction(
        &self,
        invoke: request::add_transaction::InvokeFunction,
    ) -> Result<reply::add_transaction::InvokeResponse, SequencerError> {
        unimplemented!();
    }

    async fn add_declare_transaction(
        &self,
        declare: request::add_transaction::Declare,
        token: Option<String>,
    ) -> Result<reply::add_transaction::DeclareResponse, SequencerError> {
        unimplemented!();
    }

    async fn add_deploy_account(
        &self,
        deploy: request::add_transaction::DeployAccount,
    ) -> Result<reply::add_transaction::DeployAccountResponse, SequencerError> {
        unimplemented!();
    }

    /// This is a **temporary** measure to keep the sync logic unchanged
    ///
    /// TODO remove when p2p friendly sync is implemented
    async fn head(&self) -> Result<(BlockNumber, BlockHash), SequencerError> {
        self.block_header(BlockId::Latest).await
    }

    async fn block_traces(&self, block: BlockId) -> Result<BlockTrace, SequencerError> {
        unimplemented!();
    }

    async fn transaction_trace(
        &self,
        transaction: TransactionHash,
    ) -> Result<TransactionTrace, SequencerError> {
        unimplemented!();
    }

    async fn signature(&self, block: BlockId) -> Result<reply::BlockSignature, SequencerError> {
        unimplemented!();
    }
}

#[async_trait::async_trait]
impl<T: GatewayApi + Sync + Send> GatewayApi for std::sync::Arc<T> {
    async fn pending_block(&self) -> Result<(PendingBlock, StateUpdate), SequencerError> {
        self.as_ref().pending_block().await
    }

    async fn block_header(
        &self,
        block: BlockId,
    ) -> Result<(BlockNumber, BlockHash), SequencerError> {
        self.as_ref().block_header(block).await
    }

    async fn pending_class_by_hash(
        &self,
        class_hash: ClassHash,
    ) -> Result<bytes::Bytes, SequencerError> {
        self.as_ref().pending_class_by_hash(class_hash).await
    }

    async fn pending_casm_by_hash(
        &self,
        class_hash: ClassHash,
    ) -> Result<bytes::Bytes, SequencerError> {
        self.as_ref().pending_casm_by_hash(class_hash).await
    }

    async fn transaction(
        &self,
        transaction_hash: TransactionHash,
    ) -> Result<reply::TransactionStatus, SequencerError> {
        self.as_ref().transaction(transaction_hash).await
    }

    async fn state_update_with_block(
        &self,
        block: BlockNumber,
    ) -> Result<(reply::Block, StateUpdate), SequencerError> {
        self.as_ref().state_update_with_block(block).await
    }

    async fn eth_contract_addresses(&self) -> Result<reply::EthContractAddresses, SequencerError> {
        self.as_ref().eth_contract_addresses().await
    }

    async fn add_invoke_transaction(
        &self,
        invoke: request::add_transaction::InvokeFunction,
    ) -> Result<reply::add_transaction::InvokeResponse, SequencerError> {
        self.as_ref().add_invoke_transaction(invoke).await
    }

    async fn add_declare_transaction(
        &self,
        declare: request::add_transaction::Declare,
        token: Option<String>,
    ) -> Result<reply::add_transaction::DeclareResponse, SequencerError> {
        self.as_ref().add_declare_transaction(declare, token).await
    }

    async fn add_deploy_account(
        &self,
        deploy: request::add_transaction::DeployAccount,
    ) -> Result<reply::add_transaction::DeployAccountResponse, SequencerError> {
        self.as_ref().add_deploy_account(deploy).await
    }

    async fn block_traces(&self, block: BlockId) -> Result<BlockTrace, SequencerError> {
        self.as_ref().block_traces(block).await
    }

    async fn transaction_trace(
        &self,
        transaction: TransactionHash,
    ) -> Result<TransactionTrace, SequencerError> {
        self.as_ref().transaction_trace(transaction).await
    }

    async fn signature(&self, block: BlockId) -> Result<reply::BlockSignature, SequencerError> {
        self.as_ref().signature(block).await
    }
}

/// Starknet sequencer client using REST API.
///
/// Retry is performed on __all__ types of errors __except for__
/// [Starknet specific errors](starknet_gateway_types::error::StarknetError).
///
/// Initial backoff time is 30 seconds and saturates at 10 minutes:
///
/// `backoff [secs] = min((2 ^ N) * 15, 600) [secs]`
///
/// where `N` is the consecutive retry iteration number `{1, 2, ...}`.
#[derive(Debug, Clone)]
pub struct Client {
    /// This client is internally refcounted
    inner: reqwest::Client,
    /// Starknet gateway URL.
    gateway: Url,
    /// Starknet feeder gateway URL.
    feeder_gateway: Url,
    /// Whether __read only__ requests should be retried, defaults to __true__ for production.
    /// Use [disable_retry_for_tests](Client::disable_retry_for_tests) to disable retry logic for all __read only__ requests when testing.
    retry: bool,
    /// Api key added to each request as a value for 'X-Throttling-Bypass' header.
    api_key: Option<String>,
}

impl Client {
    /// Creates a [Client] for [pathfinder_common::Chain::Mainnet].
    pub fn mainnet() -> Self {
        Self::with_base_url(Url::parse("https://alpha-mainnet.starknet.io/").unwrap()).unwrap()
    }

    /// Creates a [Client] for [pathfinder_common::Chain::GoerliTestnet].
    pub fn goerli_testnet() -> Self {
        Self::with_base_url(Url::parse("https://alpha4.starknet.io/").unwrap()).unwrap()
    }

    /// Creates a [Client] for [pathfinder_common::Chain::GoerliIntegration].
    pub fn goerli_integration() -> Self {
        Self::with_base_url(Url::parse("https://external.integration.starknet.io").unwrap())
            .unwrap()
    }

    /// Creates a [Client] for [pathfinder_common::Chain::SepoliaTestnet].
    pub fn sepolia_testnet() -> Self {
        Self::with_base_url(Url::parse("https://alpha-sepolia.starknet.io/").unwrap()).unwrap()
    }

    /// Creates a [Client] for [pathfinder_common::Chain::SepoliaIntegration].
    pub fn sepolia_integration() -> Self {
        Self::with_base_url(Url::parse("https://integration-sepolia.starknet.io/").unwrap())
            .unwrap()
    }

    /// Creates a [Client] with a shared feeder gateway and gateway base url.
    pub fn with_base_url(base: Url) -> anyhow::Result<Self> {
        let gateway = base.join("gateway")?;
        let feeder_gateway = base.join("feeder_gateway")?;
        Self::with_urls(gateway, feeder_gateway)
    }

    /// Create a Sequencer client for the given [Url]s.
    pub fn with_urls(gateway: Url, feeder_gateway: Url) -> anyhow::Result<Self> {
        metrics::register();

        Ok(Self {
            inner: reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .user_agent(pathfinder_common::consts::USER_AGENT)
                .build()?,
            gateway,
            feeder_gateway,
            retry: true,
            api_key: None,
        })
    }

    /// Sets the api key to be used for each request as a value for 'X-Throttling-Bypass' header.
    pub fn with_api_key(mut self, api_key: Option<String>) -> Self {
        self.api_key = api_key;
        self
    }

    /// Use this method to disable retry logic for all __non write__ requests when testing.
    pub fn disable_retry_for_tests(self) -> Self {
        Self {
            retry: false,
            ..self
        }
    }

    fn gateway_request(&self) -> builder::Request<'_, builder::stage::Method> {
        builder::Request::builder(&self.inner, self.gateway.clone(), self.api_key.clone())
    }

    fn feeder_gateway_request(&self) -> builder::Request<'_, builder::stage::Method> {
        builder::Request::builder(
            &self.inner,
            self.feeder_gateway.clone(),
            self.api_key.clone(),
        )
    }
}

#[async_trait::async_trait]
impl GatewayApi for Client {
    #[tracing::instrument(skip(self))]
    async fn pending_block(&self) -> Result<(PendingBlock, StateUpdate), SequencerError> {
        #[derive(Clone, Debug, serde::Deserialize)]
        struct Dto {
            pub block: PendingBlock,
            pub state_update: starknet_gateway_types::reply::StateUpdate,
        }

        let result: Dto = self
            .feeder_gateway_request()
            .get_state_update()
            .with_block(BlockId::Pending)
            .add_param("includeBlock", "true")
            .with_retry(self.retry)
            .get()
            .await?;

        Ok((result.block, result.state_update.into()))
    }

    async fn block_header(
        &self,
        block: BlockId,
    ) -> Result<(BlockNumber, BlockHash), SequencerError> {
        #[derive(serde::Deserialize)]
        #[serde(deny_unknown_fields)]
        pub struct BlockHeader {
            pub block_hash: BlockHash,
            pub block_number: BlockNumber,
        }

        let header: BlockHeader = self
            .feeder_gateway_request()
            .get_block()
            .with_block(block)
            .add_param("headerOnly", "true")
            .with_retry(self.retry)
            .get()
            .await?;

        Ok((header.block_number, header.block_hash))
    }

    /// Gets class for a particular class hash.
    #[tracing::instrument(skip(self))]
    async fn pending_class_by_hash(
        &self,
        class_hash: ClassHash,
    ) -> Result<bytes::Bytes, SequencerError> {
        self.feeder_gateway_request()
            .get_class_by_hash()
            .with_class_hash(class_hash)
            .with_block(BlockId::Pending)
            .with_retry(self.retry)
            .get_as_bytes()
            .await
    }

    /// Gets CASM for a particular class hash.
    #[tracing::instrument(skip(self))]
    async fn pending_casm_by_hash(
        &self,
        class_hash: ClassHash,
    ) -> Result<bytes::Bytes, SequencerError> {
        self.feeder_gateway_request()
            .get_compiled_class_by_class_hash()
            .with_class_hash(class_hash)
            .with_block(BlockId::Pending)
            .with_retry(self.retry)
            .get_as_bytes()
            .await
    }

    /// Gets transaction by hash.
    #[tracing::instrument(skip(self))]
    async fn transaction(
        &self,
        transaction_hash: TransactionHash,
    ) -> Result<reply::TransactionStatus, SequencerError> {
        self.feeder_gateway_request()
            .get_transaction()
            .with_transaction_hash(transaction_hash)
            .with_retry(self.retry)
            .get()
            .await
    }

    /// Gets a _block_ and the corresponding _state update_.
    ///
    /// Available since Starknet 0.12.2.
    ///
    /// This is useful because using fetching both in a single request guarantees the consistency
    /// of the block and state update information for the pending block.
    #[tracing::instrument(skip(self))]
    async fn state_update_with_block(
        &self,
        block: BlockNumber,
    ) -> Result<(reply::Block, StateUpdate), SequencerError> {
        #[derive(serde::Deserialize)]
        struct Dto {
            block: reply::Block,
            state_update: reply::StateUpdate,
        }

        let result: Dto = self
            .feeder_gateway_request()
            .get_state_update()
            .with_block(block)
            .add_param("includeBlock", "true")
            .with_retry(self.retry)
            .get()
            .await?;
        Ok((result.block, result.state_update.into()))
    }

    /// Gets addresses of the Ethereum contracts crucial to Starknet operation.
    #[tracing::instrument(skip(self))]
    async fn eth_contract_addresses(&self) -> Result<reply::EthContractAddresses, SequencerError> {
        self.feeder_gateway_request()
            .get_contract_addresses()
            .with_retry(self.retry)
            .get()
            .await
    }

    /// Adds a transaction invoking a contract.
    #[tracing::instrument(skip(self))]
    async fn add_invoke_transaction(
        &self,
        invoke: request::add_transaction::InvokeFunction,
    ) -> Result<reply::add_transaction::InvokeResponse, SequencerError> {
        // Note that we don't do retries here.
        // This method is used to proxy an add transaction operation from the JSON-RPC
        // API to the sequencer. Retries should be implemented in the JSON-RPC
        // client instead.
        self.gateway_request()
            .add_transaction()
            .with_retry(false)
            .post_with_json(&request::add_transaction::AddTransaction::Invoke(invoke))
            .await
    }

    /// Adds a transaction declaring a class.
    #[tracing::instrument(skip(self))]
    async fn add_declare_transaction(
        &self,
        declare: request::add_transaction::Declare,
        token: Option<String>,
    ) -> Result<reply::add_transaction::DeclareResponse, SequencerError> {
        // Note that we don't do retries here.
        // This method is used to proxy an add transaction operation from the JSON-RPC
        // API to the sequencer. Retries should be implemented in the JSON-RPC
        // client instead.
        self.gateway_request()
            .add_transaction()
            // mainnet requires a token (but testnet does not so its optional).
            .with_optional_token(token.as_deref())
            .with_retry(false)
            .post_with_json(&request::add_transaction::AddTransaction::Declare(declare))
            .await
    }

    #[tracing::instrument(skip(self))]
    async fn add_deploy_account(
        &self,
        deploy: request::add_transaction::DeployAccount,
    ) -> Result<reply::add_transaction::DeployAccountResponse, SequencerError> {
        // Note that we don't do retries here.
        // This method is used to proxy an add transaction operation from the JSON-RPC
        // API to the sequencer. Retries should be implemented in the JSON-RPC
        // client instead.
        self.gateway_request()
            .add_transaction()
            .with_retry(false)
            .post_with_json(&request::add_transaction::AddTransaction::DeployAccount(
                deploy,
            ))
            .await
    }

    #[tracing::instrument(skip(self))]
    async fn block_traces(&self, block: BlockId) -> Result<BlockTrace, SequencerError> {
        self.feeder_gateway_request()
            .get_block_traces()
            .with_block(block)
            .with_retry(self.retry)
            .get()
            .await
    }

    #[tracing::instrument(skip(self))]
    async fn transaction_trace(
        &self,
        transaction: TransactionHash,
    ) -> Result<TransactionTrace, SequencerError> {
        self.feeder_gateway_request()
            .get_transaction_trace()
            .with_transaction_hash(transaction)
            .with_retry(self.retry)
            .get()
            .await
    }

    #[tracing::instrument(skip(self))]
    async fn signature(&self, block: BlockId) -> Result<reply::BlockSignature, SequencerError> {
        self.feeder_gateway_request()
            .get_signature()
            .with_block(block)
            .with_retry(self.retry)
            .get()
            .await
    }
}

pub mod test_utils {
    use super::Client;
    use starknet_gateway_types::error::KnownStarknetErrorCode;

    /// Helper function which allows for easy creation of a response tuple
    /// that contains a [StarknetError](starknet_gateway_types::error::StarknetError) for a given [KnownStarknetErrorCode].
    ///
    /// The response tuple can then be used by the [setup] function.
    ///
    /// The `message` field is always an empty string.
    /// The HTTP status code for this response is always `500` (`Internal Server Error`).
    pub fn response_from(code: KnownStarknetErrorCode) -> (String, u16) {
        use starknet_gateway_types::error::StarknetError;

        let e = StarknetError {
            code: code.into(),
            message: "".to_string(),
        };
        (serde_json::to_string(&e).unwrap(), 500)
    }

    /// # Usage
    ///
    /// Use to initialize a [Client] test case. The function does one of the following things:
    ///
    /// 1. if `SEQUENCER_TESTS_LIVE_API` environment variable is set:
    ///    - creates a [Client] instance which connects to the Goerli
    ///      sequencer API
    ///
    /// 2. otherwise:
    ///    - initializes a local mock server instance with the given expected
    ///      url paths & queries and respective fixtures for replies
    ///    - creates a [Client] instance which connects to the mock server
    ///
    pub fn setup<S1, S2, const N: usize>(
        url_paths_queries_and_response_fixtures: [(S1, (S2, u16)); N],
    ) -> (Option<tokio::task::JoinHandle<()>>, Client)
    where
        S1: std::convert::AsRef<str>
            + std::fmt::Display
            + std::fmt::Debug
            + std::cmp::PartialEq
            + Send
            + Sync
            + Clone
            + 'static,
        S2: std::string::ToString + Send + Sync + Clone + 'static,
    {
        if std::env::var_os("SEQUENCER_TESTS_LIVE_API").is_some() {
            (None, Client::goerli_testnet())
        } else if std::env::var_os("SEQUENCER_TESTS_LIVE_API_INTEGRATION").is_some() {
            (None, Client::goerli_integration())
        } else {
            use warp::Filter;
            let opt_query_raw = warp::query::raw()
                .map(Some)
                .or_else(|_| async { Ok::<(Option<String>,), std::convert::Infallible>((None,)) });
            let path = warp::any().and(warp::path::full()).and(opt_query_raw).map(
                move |full_path: warp::path::FullPath, raw_query: Option<String>| {
                    let actual_full_path_and_query = match raw_query {
                        Some(some_raw_query) => {
                            format!("{}?{}", full_path.as_str(), some_raw_query.as_str())
                        }
                        None => full_path.as_str().to_owned(),
                    };

                    match url_paths_queries_and_response_fixtures
                        .iter()
                        .find(|x| x.0.as_ref() == actual_full_path_and_query)
                    {
                        Some((_, (body, status))) => http::response::Builder::new()
                            .status(*status)
                            .body(body.to_string()),
                        None => panic!(
                            "Actual url path and query {} not found in the expected {:?}",
                            actual_full_path_and_query,
                            url_paths_queries_and_response_fixtures
                                .iter()
                                .map(|(expected_path, _)| expected_path)
                                .collect::<Vec<_>>()
                        ),
                    }
                },
            );

            let (addr, serve_fut) = warp::serve(path).bind_ephemeral(([127, 0, 0, 1], 0));
            let server_handle = tokio::spawn(serve_fut);
            let client =
                Client::with_base_url(reqwest::Url::parse(&format!("http://{addr}")).unwrap())
                    .unwrap();
            (Some(server_handle), client)
        }
    }

    /// # Usage
    ///
    /// Use to initialize a [Client] test case. The function does one of the following things:
    /// - initializes a local mock server instance with the given expected
    ///   url paths & queries and respective fixtures for replies
    /// - creates a [Client] instance which connects to the mock server
    /// - replies for a particular path & query are consumed one at a time until exhausted
    ///
    /// # Panics
    ///
    /// Panics if replies for a particular path & query have been exhausted and the
    /// client still attempts to query the very same path.
    ///
    pub fn setup_with_varied_responses<const M: usize, const N: usize>(
        url_paths_queries_and_response_fixtures: [(String, [(String, u16); M]); N],
    ) -> (Option<tokio::task::JoinHandle<()>>, Client) {
        let url_paths_queries_and_response_fixtures = url_paths_queries_and_response_fixtures
            .into_iter()
            .map(|x| {
                (
                    x.0.clone(),
                    x.1.into_iter().collect::<std::collections::VecDeque<_>>(),
                )
            })
            .collect::<Vec<_>>();
        use std::sync::{Arc, Mutex};

        let url_paths_queries_and_response_fixtures =
            Arc::new(Mutex::new(url_paths_queries_and_response_fixtures));

        use warp::Filter;
        let opt_query_raw = warp::query::raw()
            .map(Some)
            .or_else(|_| async { Ok::<(Option<String>,), std::convert::Infallible>((None,)) });
        let path = warp::any().and(warp::path::full()).and(opt_query_raw).map(
            move |full_path: warp::path::FullPath, raw_query: Option<String>| {
                let actual_full_path_and_query = match raw_query {
                    Some(some_raw_query) => {
                        format!("{}?{}", full_path.as_str(), some_raw_query.as_str())
                    }
                    None => full_path.as_str().to_owned(),
                };

                let mut url_paths_queries_and_response_fixtures =
                    url_paths_queries_and_response_fixtures.lock().unwrap();

                match url_paths_queries_and_response_fixtures
                    .iter_mut()
                    .find(|x| x.0 == actual_full_path_and_query)
                {
                    Some((_, responses)) => {
                        let (body, status) =
                            responses.pop_front().expect("more responses for this path");
                        http::response::Builder::new().status(status).body(body)
                    }
                    None => panic!(
                        "Actual url path and query {} not found in the expected {:?}",
                        actual_full_path_and_query,
                        url_paths_queries_and_response_fixtures
                            .iter()
                            .map(|(expected_path, _)| expected_path)
                            .collect::<Vec<_>>()
                    ),
                }
            },
        );

        let (addr, serve_fut) = warp::serve(path).bind_ephemeral(([127, 0, 0, 1], 0));
        let server_handle = tokio::spawn(serve_fut);
        let client = Client::with_base_url(reqwest::Url::parse(&format!("http://{addr}")).unwrap())
            .unwrap()
            .disable_retry_for_tests();
        (Some(server_handle), client)
    }
}

#[cfg(test)]
mod tests {
    use super::{test_utils::*, *};
    use assert_matches::assert_matches;
    use pathfinder_common::macro_prelude::*;
    use pathfinder_common::prelude::*;
    use pathfinder_crypto::Felt;
    use starknet_gateway_test_fixtures::{testnet::*, *};
    use starknet_gateway_types::error::KnownStarknetErrorCode;
    use starknet_gateway_types::request::add_transaction::ContractDefinition;

    #[test_log::test(tokio::test)]
    async fn client_user_agent() {
        use pathfinder_common::consts::VERGEN_GIT_DESCRIBE;
        use std::convert::Infallible;
        use warp::Filter;

        let filter = warp::header::optional("user-agent").and_then(
            |user_agent: Option<String>| async move {
                let user_agent = user_agent.expect("user-agent set");
                let (name, version) = user_agent.split_once('/').unwrap();

                assert_eq!(name, "starknet-pathfinder");
                assert_eq!(version, VERGEN_GIT_DESCRIBE);

                Ok::<_, Infallible>(warp::reply::json(
                    &serde_json::json!({"block_hash": "0x0", "block_number": 0}),
                ))
            },
        );

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let (addr, run_srv) =
            warp::serve(filter).bind_with_graceful_shutdown(([127, 0, 0, 1], 0), async {
                shutdown_rx.await.ok();
            });
        let server_handle = tokio::spawn(run_srv);

        let url = format!("http://{addr}");
        let url = Url::parse(&url).unwrap();
        let client = Client::with_base_url(url).unwrap();

        let _ = client.block_header(BlockId::Latest).await;
        shutdown_tx.send(()).unwrap();
        server_handle.await.unwrap();
    }

    mod transaction {
        use super::{reply::Status, *};
        use pretty_assertions_sorted::assert_eq;

        #[tokio::test]
        async fn declare() {
            let (_jh, client) = setup([(
                "/feeder_gateway/get_transaction?transactionHash=0x587d93f2339b7f2beda040187dbfcb9e076ce4a21eb8d15ae64819718817fbe",
                (v0_9_0::transaction::INVOKE, 200)
            )]);
            assert_eq!(
                client
                    .transaction(transaction_hash!(
                        "0587d93f2339b7f2beda040187dbfcb9e076ce4a21eb8d15ae64819718817fbe"
                    ))
                    .await
                    .unwrap()
                    .status,
                Status::AcceptedOnL1
            );
        }

        #[tokio::test]
        async fn deploy() {
            let (_jh, client) = setup([(
                "/feeder_gateway/get_transaction?transactionHash=0x3d7623443283d9a0cec946492db78b06d57642a551745ddfac8d3f1f4fcc2a8",
                (v0_9_0::transaction::DEPLOY, 200)
            )]);
            assert_eq!(
                client
                    .transaction(transaction_hash!(
                        "03d7623443283d9a0cec946492db78b06d57642a551745ddfac8d3f1f4fcc2a8"
                    ))
                    .await
                    .unwrap()
                    .status,
                Status::AcceptedOnL1
            );
        }

        #[tokio::test]
        async fn invoke() {
            let (_jh, client) = setup([(
                "/feeder_gateway/get_transaction?transactionHash=0x587d93f2339b7f2beda040187dbfcb9e076ce4a21eb8d15ae64819718817fbe",
                (v0_9_0::transaction::INVOKE, 200)
            )]);
            assert_eq!(
                client
                    .transaction(transaction_hash!(
                        "0587d93f2339b7f2beda040187dbfcb9e076ce4a21eb8d15ae64819718817fbe"
                    ))
                    .await
                    .unwrap()
                    .status,
                Status::AcceptedOnL1
            );
        }

        #[tokio::test]
        async fn invalid_hash() {
            let (_jh, client) = setup([(
                format!(
                    "/feeder_gateway/get_transaction?transactionHash={}",
                    INVALID_TX_HASH.0.to_hex_str()
                ),
                (
                    r#"{"status": "NOT_RECEIVED", "finality_status": "NOT_RECEIVED"}"#,
                    200,
                ),
            )]);
            assert_eq!(
                client.transaction(INVALID_TX_HASH).await.unwrap().status,
                Status::NotReceived,
            );
        }
    }

    #[tokio::test]
    async fn eth_contract_addresses() {
        let (_jh, client) = setup([(
            "/feeder_gateway/get_contract_addresses",
            (
                r#"{"Starknet":"0xde29d060d45901fb19ed6c6e959eb22d8626708e","GpsStatementVerifier":"0xab43ba48c9edf4c2c4bb01237348d1d7b28ef168"}"#,
                200,
            ),
        )]);
        client.eth_contract_addresses().await.unwrap();
    }

    mod add_transaction {
        use super::*;
        use pathfinder_common::ContractAddress;
        use starknet_gateway_types::request::{
            add_transaction::CairoContractDefinition,
            contract::{EntryPointType, SelectorAndOffset},
        };
        use std::collections::HashMap;

        mod invoke {
            use super::*;

            fn inputs() -> (
                TransactionVersion,
                Fee,
                Vec<TransactionSignatureElem>,
                TransactionNonce,
                ContractAddress,
                Vec<CallParam>,
            ) {
                (
                    TransactionVersion::ONE,
                    fee!("4F388496839"),
                    vec![
                        transaction_signature_elem!(
                            "0x07dd3a55d94a0de6f3d6c104d7e6c88ec719a82f4e2bbc12587c8c187584d3d5"
                        ),
                        transaction_signature_elem!(
                            "0x071456dded17015d1234779889d78f3e7c763ddcfd2662b19e7843c7542614f8"
                        ),
                    ],
                    transaction_nonce!("0x1"),
                    contract_address!(
                        "0x023371b227eaecd8e8920cd429357edddd2cd0f3fee6abaacca08d3ab82a7cdd"
                    ),
                    vec![
                        call_param!("0x1"),
                        call_param!(
                            "0677bb1cdc050e8d63855e8743ab6e09179138def390676cc03c484daf112ba1"
                        ),
                        call_param!(
                            "0362398bec32bc0ebb411203221a35a0301193a96f317ebe5e40be9f60d15320"
                        ),
                        CallParam(Felt::ZERO),
                        call_param!("0x1"),
                        call_param!("0x1"),
                        call_param!("0x2b"),
                        CallParam(Felt::ZERO),
                    ],
                )
            }

            #[tokio::test]
            async fn v0_is_deprecated() {
                use request::add_transaction::{InvokeFunction, InvokeFunctionV0V1};

                let (_jh, client) = setup([(
                    "/gateway/add_transaction",
                    response_from(KnownStarknetErrorCode::DeprecatedTransaction),
                )]);
                let (_, fee, sig, nonce, addr, call) = inputs();
                let invoke = InvokeFunction::V0(InvokeFunctionV0V1 {
                    max_fee: fee,
                    signature: sig,
                    nonce: Some(nonce),
                    sender_address: addr,
                    entry_point_selector: None,
                    calldata: call,
                });

                let error = client.add_invoke_transaction(invoke).await.unwrap_err();
                assert_matches!(
                    error,
                    SequencerError::StarknetError(e) => assert_eq!(e.code, KnownStarknetErrorCode::DeprecatedTransaction.into())
                );
            }

            #[tokio::test]
            async fn successful() {
                use request::add_transaction::{InvokeFunction, InvokeFunctionV0V1};

                let (_jh, client) = setup([(
                    "/gateway/add_transaction",
                    (
                        r#"{"code":"TRANSACTION_RECEIVED","transaction_hash":"0x0389DD0629F42176CC8B6C43ACEFC0713D0064ECDFC0470E0FC179F53421A38B"}"#,
                        200,
                    ),
                )]);
                // test with values dumped from `starknet invoke` for a test contract
                let (_, fee, sig, nonce, addr, call) = inputs();
                let invoke = InvokeFunction::V1(InvokeFunctionV0V1 {
                    max_fee: fee,
                    signature: sig,
                    nonce: Some(nonce),
                    sender_address: addr,
                    entry_point_selector: None,
                    calldata: call,
                });
                client.add_invoke_transaction(invoke).await.unwrap();
            }
        }

        mod declare {
            use starknet_gateway_types::request::{
                add_transaction::SierraContractDefinition, contract::SelectorAndFunctionIndex,
            };

            use super::*;

            #[tokio::test]
            async fn v0_is_deprecated() {
                use request::add_transaction::{Declare, DeclareV0V1V2};

                let (_jh, client) = setup([(
                    "/gateway/add_transaction",
                    response_from(KnownStarknetErrorCode::DeprecatedTransaction),
                )]);

                let declare = Declare::V0(DeclareV0V1V2 {
                    version: TransactionVersion::ZERO,
                    max_fee: Fee(Felt::ZERO),
                    signature: vec![],
                    contract_class: ContractDefinition::Cairo(cairo_contract_class_from_fixture()),
                    sender_address: contract_address!("0x1"),
                    nonce: TransactionNonce::ZERO,
                    compiled_class_hash: None,
                });
                let error = client
                    .add_declare_transaction(declare, None)
                    .await
                    .unwrap_err();
                assert_matches!(
                    error,
                    SequencerError::StarknetError(e) => assert_eq!(e.code, KnownStarknetErrorCode::DeprecatedTransaction.into())
                );
            }

            #[tokio::test]
            async fn successful_v1() {
                use request::add_transaction::{Declare, DeclareV0V1V2};

                let (_jh, client) = setup([(
                    "/gateway/add_transaction",
                    (
                        r#"{"code": "TRANSACTION_RECEIVED",
                            "transaction_hash": "0x77ccba4df42cf0f74a8eb59a96d7880fae371edca5d000ca5f9985652c8a8ed",
                            "class_hash": "0x711941b11a8236b8cca42b664e19342ac7300abb1dc44957763cb65877c2708"}"#,
                        200,
                    ),
                )]);

                let declare = Declare::V1(DeclareV0V1V2 {
                    version: TransactionVersion::ONE,
                    max_fee: fee!("0xFFFF"),
                    signature: vec![],
                    contract_class: ContractDefinition::Cairo(cairo_contract_class_from_fixture()),
                    sender_address: contract_address!("0x1"),
                    nonce: TransactionNonce(Felt::ZERO),
                    compiled_class_hash: None,
                });

                client.add_declare_transaction(declare, None).await.unwrap();
            }

            fn sierra_contract_class_from_fixture() -> SierraContractDefinition {
                let sierra_class =
                    starknet_gateway_test_fixtures::class_definitions::CAIRO_1_0_0_ALPHA6_SIERRA;
                let mut sierra_class =
                    serde_json::from_slice::<serde_json::Value>(sierra_class).unwrap();
                let sierra_program = sierra_class.get_mut("sierra_program").unwrap().take();
                let sierra_program = serde_json::from_value::<Vec<Felt>>(sierra_program).unwrap();
                let mut gzip_encoder =
                    flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
                serde_json::to_writer(&mut gzip_encoder, &sierra_program).unwrap();
                let sierra_program = gzip_encoder.finish().unwrap();
                let sierra_program = base64::encode(sierra_program);

                let mut entry_points = sierra_class.get_mut("entry_points_by_type").unwrap().take();

                let mut entry_points_by_type: HashMap<
                    EntryPointType,
                    Vec<SelectorAndFunctionIndex>,
                > = Default::default();
                entry_points_by_type.insert(
                    EntryPointType::Constructor,
                    serde_json::from_value::<Vec<SelectorAndFunctionIndex>>(
                        entry_points.get_mut("CONSTRUCTOR").unwrap().take(),
                    )
                    .unwrap(),
                );
                entry_points_by_type.insert(
                    EntryPointType::External,
                    serde_json::from_value::<Vec<SelectorAndFunctionIndex>>(
                        entry_points.get_mut("EXTERNAL").unwrap().take(),
                    )
                    .unwrap(),
                );
                entry_points_by_type.insert(
                    EntryPointType::L1Handler,
                    serde_json::from_value::<Vec<SelectorAndFunctionIndex>>(
                        entry_points.get_mut("L1_HANDLER").unwrap().take(),
                    )
                    .unwrap(),
                );

                SierraContractDefinition {
                    sierra_program,
                    contract_class_version: "0.1.0".into(),
                    abi: "trust the contract developer".into(),
                    entry_points_by_type,
                }
            }

            #[tokio::test]
            async fn successful_v2() {
                use request::add_transaction::{Declare, DeclareV0V1V2};

                let (_jh, client) = setup([(
                    "/gateway/add_transaction",
                    (
                        r#"{"code": "TRANSACTION_RECEIVED",
                            "transaction_hash": "0x77ccba4df42cf0f74a8eb59a96d7880fae371edca5d000ca5f9985652c8a8ed",
                            "class_hash": "0x711941b11a8236b8cca42b664e19342ac7300abb1dc44957763cb65877c2708"}"#,
                        200,
                    ),
                )]);

                let declare = Declare::V2(DeclareV0V1V2 {
                    version: TransactionVersion::TWO,
                    max_fee: fee!("0xffff"),
                    signature: vec![],
                    contract_class: ContractDefinition::Sierra(sierra_contract_class_from_fixture()),
                    sender_address: contract_address!("0x1"),
                    nonce: TransactionNonce::ZERO,
                    compiled_class_hash: Some(casm_hash!(
                        "0x5bcd45099caf3dca6c0c0f6697698c90eebf02851acbbaf911186b173472fcc"
                    )),
                });

                client.add_declare_transaction(declare, None).await.unwrap();
            }
        }

        #[tokio::test]
        async fn test_deploy_account() {
            use request::add_transaction::{DeployAccount, DeployAccountV0V1};

            let (_jh, client) = setup([(
                "/gateway/add_transaction",
                (v0_10_1::add_transaction::DEPLOY_ACCOUNT_RESPONSE, 200),
            )]);

            let request = DeployAccount::V1(DeployAccountV0V1 {
                max_fee: fee!("0xbf391377813"),
                signature: vec![
                    transaction_signature_elem!(
                        "0x70872c11ad15910fe3d0e9375c10d1794d77cd866aa6733e31a9736559ac92b"
                    ),
                    transaction_signature_elem!(
                        "0x4c9140cb8afeebc0cde2a70d11b71ec764a4d0c6b2c33356bb7d5f7c734f5e1"
                    ),
                ],
                nonce: transaction_nonce!("0x0"),
                class_hash: class_hash!(
                    "0x1fac3074c9d5282f0acc5c69a4781a1c711efea5e73c550c5d9fb253cf7fd3d"
                ),
                contract_address_salt: contract_address_salt!(
                    "0x6d44a6aecb4339e23a9619355f101cf3cb9baec289fcd9fd51486655c1bb8a8"
                ),
                constructor_calldata: vec![call_param!(
                    "0x7eda1c9b366a008b8697fe9d6bad040818ffb27f8615966c29de33e523e9e35"
                )],
            });

            let res = client
                .add_deploy_account(request)
                .await
                .expect("DEPLOY_ACCOUNT response");

            let expected = reply::add_transaction::DeployAccountResponse {
                code: "TRANSACTION_RECEIVED".to_string(),
                transaction_hash: transaction_hash!(
                    "06dac1655b34e52a449cfe961188f7cc2b1496bcd36706cedf4935567be29d5b"
                ),
            };

            assert_eq!(res, expected);
        }

        /// Return a contract definition that was dumped from a `starknet deploy`.
        fn cairo_contract_class_from_fixture() -> CairoContractDefinition {
            let json = starknet_gateway_test_fixtures::class_definitions::CONTRACT_DEFINITION;
            let json: serde_json::Value = serde_json::from_slice(json).unwrap();
            let program = &json["program"];

            // Program is expected to be a gzip-compressed then base64 encoded representation of the JSON.
            let mut gzip_encoder =
                flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
            serde_json::to_writer(&mut gzip_encoder, program).unwrap();
            let compressed_program = gzip_encoder.finish().unwrap();
            let program = base64::encode(compressed_program);

            let entry_points_by_type: HashMap<EntryPointType, Vec<SelectorAndOffset>> =
                HashMap::from([
                    (EntryPointType::Constructor, vec![]),
                    (
                        EntryPointType::External,
                        vec![
                            SelectorAndOffset {
                                offset: byte_code_offset!("0x3a"),
                                selector: entry_point!("0362398bec32bc0ebb411203221a35a0301193a96f317ebe5e40be9f60d15320"),
                            },
                            SelectorAndOffset {
                                offset: byte_code_offset!("0x5b"),
                                selector: entry_point!("039e11d48192e4333233c7eb19d10ad67c362bb28580c604d67884c85da39695"),
                            },
                        ],
                    ),
                    (EntryPointType::L1Handler, vec![]),
                ]);
            CairoContractDefinition {
                program,
                entry_points_by_type,
                abi: Some(json["contract_definition"]["abi"].clone()),
            }
        }

        mod deploy_token {
            use super::*;
            use http::StatusCode;
            use std::collections::HashMap;
            use warp::{http::Response, Filter};

            const EXPECTED_TOKEN: &str = "magic token value";
            const EXPECTED_ERROR_MESSAGE: &str = "error message";

            fn test_server() -> (tokio::task::JoinHandle<()>, std::net::SocketAddr) {
                fn token_check(params: HashMap<String, String>) -> impl warp::Reply {
                    match params.get("token") {
                        Some(token) if token == EXPECTED_TOKEN => Response::builder().status(StatusCode::OK).body(serde_json::to_vec(&serde_json::json!({
                            "code": "TRANSACTION_ACCEPTED",
                            "transaction_hash": "0x57ed4b4c76a1ca0ba044a654dd3ee2d0d3e550343d739350a22aacdd524110d",
                            "class_hash":"0x3926aea98213ec34fe9783d803237d221c54c52344422e1f4942a5b340fa6ad"
                        })).unwrap()),
                        _ => Response::builder().status(StatusCode::INTERNAL_SERVER_ERROR).body(serde_json::to_vec(&serde_json::json!({
                            "code": "StarknetErrorCode.NON_PERMITTED_CONTRACT",
                            "message": EXPECTED_ERROR_MESSAGE,
                        })).unwrap())
                    }
                }

                let route = warp::any()
                    .and(warp::query::<HashMap<String, String>>())
                    .map(token_check);
                let (addr, run_srv) = warp::serve(route).bind_ephemeral(([127, 0, 0, 1], 0));
                let server_handle = tokio::spawn(run_srv);
                (server_handle, addr)
            }

            #[test_log::test(tokio::test)]
            async fn test_token_is_passed_to_sequencer_api() {
                use request::add_transaction::{Declare, DeclareV0V1V2};

                let (_jh, addr) = test_server();
                let mut url = reqwest::Url::parse("http://localhost/").unwrap();
                url.set_port(Some(addr.port())).unwrap();
                let client = Client::with_base_url(url).unwrap();

                let declare = Declare::V0(DeclareV0V1V2 {
                    version: TransactionVersion::ZERO,
                    max_fee: Fee::ZERO,
                    signature: vec![],
                    contract_class: ContractDefinition::Cairo(CairoContractDefinition {
                        program: "".to_owned(),
                        entry_points_by_type: HashMap::new(),
                        abi: None,
                    }),
                    sender_address: ContractAddress::ZERO,
                    nonce: TransactionNonce::ZERO,
                    compiled_class_hash: None,
                });

                client
                    .add_declare_transaction(declare, Some(EXPECTED_TOKEN.to_owned()))
                    .await
                    .unwrap();
            }

            #[test_log::test(tokio::test)]
            async fn test_declare_fails_with_no_token() {
                use request::add_transaction::{Declare, DeclareV0V1V2};

                let (_jh, addr) = test_server();
                let mut url = reqwest::Url::parse("http://localhost/").unwrap();
                url.set_port(Some(addr.port())).unwrap();
                let client = Client::with_base_url(url).unwrap();

                let declare = Declare::V0(DeclareV0V1V2 {
                    version: TransactionVersion::ZERO,
                    max_fee: Fee::ZERO,
                    signature: vec![],
                    contract_class: ContractDefinition::Cairo(CairoContractDefinition {
                        program: "".to_owned(),
                        entry_points_by_type: HashMap::new(),
                        abi: None,
                    }),
                    sender_address: ContractAddress::ZERO,
                    nonce: TransactionNonce::ZERO,
                    compiled_class_hash: None,
                });

                let err = client
                    .add_declare_transaction(declare, None)
                    .await
                    .unwrap_err();

                assert_matches!(err, SequencerError::StarknetError(se) => {
                        assert_eq!(se.code, KnownStarknetErrorCode::NotPermittedContract.into());
                        assert_eq!(se.message, EXPECTED_ERROR_MESSAGE);
                });
            }
        }
    }

    mod block_header {
        use super::*;

        const REPLY: &str = r#"{
            "block_hash": "0x6a2755817d86ade81ed0fea2eaf23d94264e2f25aff43ecb2e5000bf3ec28b7",
            "block_number": 9703
        }"#;

        #[test_log::test(tokio::test)]
        async fn success_by_number() {
            let (_jh, client) = setup([(
                "/feeder_gateway/get_block?blockNumber=9703&headerOnly=true",
                (REPLY.to_owned(), 200),
            )]);

            client
                .block_header(BlockId::Number(BlockNumber::new_or_panic(9703)))
                .await
                .unwrap();
        }

        #[test_log::test(tokio::test)]
        async fn success_by_hash() {
            let (_jh, client) = setup([(
                "/feeder_gateway/get_block?blockHash=0x6a2755817d86ade81ed0fea2eaf23d94264e2f25aff43ecb2e5000bf3ec28b7&headerOnly=true",
                (REPLY.to_owned(), 200),
            )]);

            client
                .block_header(
                    block_hash!(
                        "0x6a2755817d86ade81ed0fea2eaf23d94264e2f25aff43ecb2e5000bf3ec28b7"
                    )
                    .into(),
                )
                .await
                .unwrap();
        }

        #[test_log::test(tokio::test)]
        async fn block_not_found() {
            const BLOCK_NUMBER: u64 = 99999999;
            let (_jh, client) = setup([(
                format!("/feeder_gateway/get_block?blockNumber={BLOCK_NUMBER}&headerOnly=true",),
                response_from(KnownStarknetErrorCode::BlockNotFound),
            )]);
            let error = client
                .block_header(BlockNumber::new_or_panic(BLOCK_NUMBER).into())
                .await
                .unwrap_err();
            assert_matches!(
                error,
                SequencerError::StarknetError(e) => assert_eq!(e.code, KnownStarknetErrorCode::BlockNotFound.into())
            );
        }
    }

    mod pending_block {
        use super::*;

        #[test_log::test(tokio::test)]
        async fn success() {
            let (_jh, client) = setup([(
                "/feeder_gateway/get_state_update?blockNumber=pending&includeBlock=true",
                (
                    starknet_gateway_test_fixtures::v0_13_1::state_update_with_block::SEPOLIA_INTEGRATION_PENDING,
                    200,
                ),
            )]);

            client.pending_block().await.unwrap();
        }

        #[test_log::test(tokio::test)]
        async fn block_not_found() {
            let (_jh, client) = setup([(
                "/feeder_gateway/get_state_update?blockNumber=pending&includeBlock=true",
                response_from(KnownStarknetErrorCode::BlockNotFound),
            )]);
            let error = client.pending_block().await.unwrap_err();
            assert_matches!(
                error,
                SequencerError::StarknetError(e) => assert_eq!(e.code, KnownStarknetErrorCode::BlockNotFound.into())
            );
        }
    }

    mod state_update_with_block {
        use super::*;

        #[test_log::test(tokio::test)]
        async fn success() {
            let (_jh, client) = setup([(
                "/feeder_gateway/get_state_update?blockNumber=9703&includeBlock=true",
                (
                    starknet_gateway_test_fixtures::v0_13_1::state_update_with_block::SEPOLIA_INTEGRATION_NUMBER_9703,
                    200,
                ),
            )]);

            client
                .state_update_with_block(BlockNumber::new_or_panic(9703))
                .await
                .unwrap();
        }

        #[test_log::test(tokio::test)]
        async fn block_not_found() {
            const BLOCK_NUMBER: u64 = 99999999;
            let (_jh, client) = setup([(
                format!(
                    "/feeder_gateway/get_state_update?blockNumber={BLOCK_NUMBER}&includeBlock=true"
                ),
                response_from(KnownStarknetErrorCode::BlockNotFound),
            )]);
            let error = client
                .state_update_with_block(BlockNumber::new_or_panic(BLOCK_NUMBER))
                .await
                .unwrap_err();
            assert_matches!(
                error,
                SequencerError::StarknetError(e) => assert_eq!(e.code, KnownStarknetErrorCode::BlockNotFound.into())
            );
        }
    }

    mod signature {
        use super::*;

        #[tokio::test]
        async fn success() {
            let (_jh, client) = setup([(
                "/feeder_gateway/get_signature?blockNumber=350000",
                (
                    starknet_gateway_test_fixtures::v0_12_2::signature::BLOCK_350000,
                    200,
                ),
            )]);

            client
                .signature(BlockId::Number(BlockNumber::new_or_panic(350000)))
                .await
                .unwrap();
        }
    }
}
