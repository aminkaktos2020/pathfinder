//! Common data structures used by the JSON-RPC API methods.

pub(crate) mod class;
pub use class::*;
use pathfinder_common::{ResourceAmount, ResourcePricePerUnit};
use serde_with::serde_as;
pub mod syncing;

#[derive(Copy, Clone, Debug, Default, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct ResourceBounds {
    pub l1_gas: ResourceBound,
    pub l2_gas: ResourceBound,
}

impl From<ResourceBounds> for pathfinder_common::transaction::ResourceBounds {
    fn from(resource_bounds: ResourceBounds) -> Self {
        Self {
            l1_gas: resource_bounds.l1_gas.into(),
            l2_gas: resource_bounds.l2_gas.into(),
        }
    }
}

#[serde_as]
#[derive(Copy, Clone, Debug, Default, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct ResourceBound {
    #[serde_as(as = "pathfinder_serde::ResourceAmountAsHexStr")]
    pub max_amount: ResourceAmount,
    #[serde_as(as = "pathfinder_serde::ResourcePricePerUnitAsHexStr")]
    pub max_price_per_unit: ResourcePricePerUnit,
}

impl From<ResourceBound> for pathfinder_common::transaction::ResourceBound {
    fn from(resource_bound: ResourceBound) -> Self {
        Self {
            max_amount: resource_bound.max_amount,
            max_price_per_unit: resource_bound.max_price_per_unit,
        }
    }
}

#[derive(Copy, Clone, Debug, Default, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub enum DataAvailabilityMode {
    #[default]
    L1,
    L2,
}

impl From<DataAvailabilityMode> for pathfinder_common::transaction::DataAvailabilityMode {
    fn from(data_availability_mode: DataAvailabilityMode) -> Self {
        match data_availability_mode {
            DataAvailabilityMode::L1 => Self::L1,
            DataAvailabilityMode::L2 => Self::L2,
        }
    }
}

impl From<DataAvailabilityMode> for starknet_api::data_availability::DataAvailabilityMode {
    fn from(value: DataAvailabilityMode) -> Self {
        match value {
            DataAvailabilityMode::L1 => Self::L1,
            DataAvailabilityMode::L2 => Self::L2,
        }
    }
}

/// Groups all strictly input types of the RPC API.
pub mod request {
    use pathfinder_common::{
        AccountDeploymentDataElem, CallParam, CasmHash, ChainId, ClassHash, ContractAddress,
        ContractAddressSalt, EntryPoint, Fee, PaymasterDataElem, Tip, TransactionNonce,
        TransactionSignatureElem, TransactionVersion,
    };
    use serde::Deserialize;
    use serde_with::serde_as;

    /// "Broadcasted" L2 transaction in requests the RPC API.
    ///
    /// "Broadcasted" transactions represent the data required to submit a new transaction.
    /// Notably, it's missing values computed during execution of the transaction, like
    /// transaction_hash or contract_address.
    #[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
    #[cfg_attr(any(test, feature = "rpc-full-serde"), derive(serde::Serialize))]
    #[serde(deny_unknown_fields, tag = "type")]
    pub enum BroadcastedTransaction {
        #[serde(rename = "DECLARE")]
        Declare(BroadcastedDeclareTransaction),
        #[serde(rename = "INVOKE")]
        Invoke(BroadcastedInvokeTransaction),
        #[serde(rename = "DEPLOY_ACCOUNT")]
        DeployAccount(BroadcastedDeployAccountTransaction),
    }

    impl BroadcastedTransaction {
        pub fn into_invoke(self) -> Option<BroadcastedInvokeTransaction> {
            match self {
                Self::Invoke(x) => Some(x),
                _ => None,
            }
        }

        pub fn into_declare(self) -> Option<BroadcastedDeclareTransaction> {
            match self {
                Self::Declare(x) => Some(x),
                _ => None,
            }
        }

        pub fn into_deploy_account(self) -> Option<BroadcastedDeployAccountTransaction> {
            match self {
                Self::DeployAccount(x) => Some(x),
                _ => None,
            }
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    #[cfg_attr(
        any(test, feature = "rpc-full-serde"),
        derive(serde::Serialize),
        serde(untagged)
    )]
    pub enum BroadcastedDeclareTransaction {
        V0(BroadcastedDeclareTransactionV0),
        V1(BroadcastedDeclareTransactionV1),
        V2(BroadcastedDeclareTransactionV2),
        V3(BroadcastedDeclareTransactionV3),
    }

    impl<'de> serde::Deserialize<'de> for BroadcastedDeclareTransaction {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            use serde::de;

            #[serde_as]
            #[derive(serde::Deserialize)]
            struct Version {
                pub version: TransactionVersion,
            }

            let v = serde_json::Value::deserialize(deserializer)?;
            let version = Version::deserialize(&v).map_err(de::Error::custom)?;
            match version.version.without_query_version() {
                0 => Ok(Self::V0(
                    BroadcastedDeclareTransactionV0::deserialize(&v).map_err(de::Error::custom)?,
                )),
                1 => Ok(Self::V1(
                    BroadcastedDeclareTransactionV1::deserialize(&v).map_err(de::Error::custom)?,
                )),
                2 => Ok(Self::V2(
                    BroadcastedDeclareTransactionV2::deserialize(&v).map_err(de::Error::custom)?,
                )),
                3 => Ok(Self::V3(
                    BroadcastedDeclareTransactionV3::deserialize(&v).map_err(de::Error::custom)?,
                )),
                _v => Err(de::Error::custom("version must be 0, 1, 2 or 3")),
            }
        }
    }

    #[serde_as]
    #[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
    #[cfg_attr(any(test, feature = "rpc-full-serde"), derive(serde::Serialize))]
    #[serde(deny_unknown_fields)]
    pub struct BroadcastedDeclareTransactionV0 {
        // BROADCASTED_TXN_COMMON_PROPERTIES: ideally this should just be included
        // here in a flattened struct, but `flatten` doesn't work with
        // `deny_unknown_fields`: https://serde.rs/attr-flatten.html#struct-flattening
        pub max_fee: Fee,
        pub version: TransactionVersion,
        pub signature: Vec<TransactionSignatureElem>,

        pub contract_class: super::CairoContractClass,
        pub sender_address: ContractAddress,
    }

    #[serde_as]
    #[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
    #[cfg_attr(any(test, feature = "rpc-full-serde"), derive(serde::Serialize))]
    #[serde(deny_unknown_fields)]
    pub struct BroadcastedDeclareTransactionV1 {
        // BROADCASTED_TXN_COMMON_PROPERTIES: ideally this should just be included
        // here in a flattened struct, but `flatten` doesn't work with
        // `deny_unknown_fields`: https://serde.rs/attr-flatten.html#struct-flattening
        pub max_fee: Fee,
        pub version: TransactionVersion,
        pub signature: Vec<TransactionSignatureElem>,
        pub nonce: TransactionNonce,

        pub contract_class: super::CairoContractClass,
        pub sender_address: ContractAddress,
    }

    #[serde_as]
    #[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
    #[cfg_attr(any(test, feature = "rpc-full-serde"), derive(serde::Serialize))]
    #[serde(deny_unknown_fields)]
    pub struct BroadcastedDeclareTransactionV2 {
        // BROADCASTED_TXN_COMMON_PROPERTIES: ideally this should just be included
        // here in a flattened struct, but `flatten` doesn't work with
        // `deny_unknown_fields`: https://serde.rs/attr-flatten.html#struct-flattening
        pub max_fee: Fee,
        pub version: TransactionVersion,
        pub signature: Vec<TransactionSignatureElem>,
        pub nonce: TransactionNonce,

        pub compiled_class_hash: CasmHash,
        pub contract_class: super::SierraContractClass,
        pub sender_address: ContractAddress,
    }

    #[serde_as]
    #[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
    #[cfg_attr(any(test, feature = "rpc-full-serde"), derive(serde::Serialize))]
    #[serde(deny_unknown_fields)]
    pub struct BroadcastedDeclareTransactionV3 {
        pub version: TransactionVersion,
        pub signature: Vec<TransactionSignatureElem>,
        pub nonce: TransactionNonce,
        pub resource_bounds: super::ResourceBounds,
        #[serde_as(as = "pathfinder_serde::TipAsHexStr")]
        pub tip: Tip,
        pub paymaster_data: Vec<PaymasterDataElem>,
        pub account_deployment_data: Vec<AccountDeploymentDataElem>,
        pub nonce_data_availability_mode: super::DataAvailabilityMode,
        pub fee_data_availability_mode: super::DataAvailabilityMode,

        pub compiled_class_hash: CasmHash,
        pub contract_class: super::SierraContractClass,
        pub sender_address: ContractAddress,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    #[cfg_attr(
        any(test, feature = "rpc-full-serde"),
        derive(serde::Serialize),
        serde(untagged)
    )]
    pub enum BroadcastedDeployAccountTransaction {
        V0V1(BroadcastedDeployAccountTransactionV0V1),
        V3(BroadcastedDeployAccountTransactionV3),
    }

    impl BroadcastedDeployAccountTransaction {
        pub fn deployed_contract_address(&self) -> ContractAddress {
            match self {
                Self::V0V1(tx) => tx.deployed_contract_address(),
                Self::V3(tx) => tx.deployed_contract_address(),
            }
        }
    }

    impl<'de> serde::Deserialize<'de> for BroadcastedDeployAccountTransaction {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            use serde::de;

            #[serde_as]
            #[derive(serde::Deserialize)]
            struct Version {
                pub version: TransactionVersion,
            }

            let v = serde_json::Value::deserialize(deserializer)?;
            let version = Version::deserialize(&v).map_err(de::Error::custom)?;
            match version.version.without_query_version() {
                0 | 1 => Ok(Self::V0V1(
                    BroadcastedDeployAccountTransactionV0V1::deserialize(&v)
                        .map_err(de::Error::custom)?,
                )),
                3 => Ok(Self::V3(
                    BroadcastedDeployAccountTransactionV3::deserialize(&v)
                        .map_err(de::Error::custom)?,
                )),
                _v => Err(de::Error::custom("version must be 0 or 1")),
            }
        }
    }

    #[serde_as]
    #[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
    #[cfg_attr(any(test, feature = "rpc-full-serde"), derive(serde::Serialize))]
    #[serde(deny_unknown_fields)]
    pub struct BroadcastedDeployAccountTransactionV0V1 {
        // Fields from BROADCASTED_TXN_COMMON_PROPERTIES
        pub version: TransactionVersion,
        pub max_fee: Fee,
        pub signature: Vec<TransactionSignatureElem>,
        pub nonce: TransactionNonce,

        // Fields from DEPLOY_ACCOUNT_TXN_PROPERTIES
        pub contract_address_salt: ContractAddressSalt,
        pub constructor_calldata: Vec<CallParam>,
        pub class_hash: ClassHash,
    }

    impl BroadcastedDeployAccountTransactionV0V1 {
        pub fn deployed_contract_address(&self) -> ContractAddress {
            ContractAddress::deployed_contract_address(
                self.constructor_calldata.iter().copied(),
                &self.contract_address_salt,
                &self.class_hash,
            )
        }
    }

    #[serde_as]
    #[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
    #[cfg_attr(any(test, feature = "rpc-full-serde"), derive(serde::Serialize))]
    #[serde(deny_unknown_fields)]
    pub struct BroadcastedDeployAccountTransactionV3 {
        pub version: TransactionVersion,
        pub signature: Vec<TransactionSignatureElem>,
        pub nonce: TransactionNonce,
        pub resource_bounds: super::ResourceBounds,
        #[serde_as(as = "pathfinder_serde::TipAsHexStr")]
        pub tip: Tip,
        pub paymaster_data: Vec<PaymasterDataElem>,
        pub nonce_data_availability_mode: super::DataAvailabilityMode,
        pub fee_data_availability_mode: super::DataAvailabilityMode,

        pub contract_address_salt: ContractAddressSalt,
        pub constructor_calldata: Vec<CallParam>,
        pub class_hash: ClassHash,
    }

    impl BroadcastedDeployAccountTransactionV3 {
        pub fn deployed_contract_address(&self) -> ContractAddress {
            ContractAddress::deployed_contract_address(
                self.constructor_calldata.iter().copied(),
                &self.contract_address_salt,
                &self.class_hash,
            )
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    #[cfg_attr(
        any(test, feature = "rpc-full-serde"),
        derive(serde::Serialize),
        serde(untagged)
    )]
    pub enum BroadcastedInvokeTransaction {
        V0(BroadcastedInvokeTransactionV0),
        V1(BroadcastedInvokeTransactionV1),
        V3(BroadcastedInvokeTransactionV3),
    }

    impl BroadcastedInvokeTransaction {
        pub fn into_v1(self) -> Option<BroadcastedInvokeTransactionV1> {
            match self {
                Self::V1(x) => Some(x),
                _ => None,
            }
        }

        pub fn into_v0(self) -> Option<BroadcastedInvokeTransactionV0> {
            match self {
                Self::V0(x) => Some(x),
                _ => None,
            }
        }
    }

    impl<'de> Deserialize<'de> for BroadcastedInvokeTransaction {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            use serde::de;

            #[serde_as]
            #[derive(Deserialize)]
            struct Version {
                pub version: TransactionVersion,
            }

            let v = serde_json::Value::deserialize(deserializer)?;
            let version = Version::deserialize(&v).map_err(de::Error::custom)?;
            match version.version.without_query_version() {
                0 => Ok(Self::V0(
                    BroadcastedInvokeTransactionV0::deserialize(&v).map_err(de::Error::custom)?,
                )),
                1 => Ok(Self::V1(
                    BroadcastedInvokeTransactionV1::deserialize(&v).map_err(de::Error::custom)?,
                )),
                3 => Ok(Self::V3(
                    BroadcastedInvokeTransactionV3::deserialize(&v).map_err(de::Error::custom)?,
                )),
                _ => Err(de::Error::custom("version must be 0, 1 or 3")),
            }
        }
    }

    #[serde_as]
    #[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
    #[cfg_attr(any(test, feature = "rpc-full-serde"), derive(serde::Serialize))]
    #[serde(deny_unknown_fields)]
    pub struct BroadcastedInvokeTransactionV0 {
        pub version: TransactionVersion,

        // BROADCASTED_TXN_COMMON_PROPERTIES: ideally this should just be included
        // here in a flattened struct, but `flatten` doesn't work with
        // `deny_unknown_fields`: https://serde.rs/attr-flatten.html#struct-flattening
        pub max_fee: Fee,
        pub signature: Vec<TransactionSignatureElem>,

        pub contract_address: ContractAddress,
        pub entry_point_selector: EntryPoint,
        pub calldata: Vec<CallParam>,
    }

    #[serde_as]
    #[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
    #[cfg_attr(any(test, feature = "rpc-full-serde"), derive(serde::Serialize))]
    #[serde(deny_unknown_fields)]
    pub struct BroadcastedInvokeTransactionV1 {
        pub version: TransactionVersion,

        // BROADCASTED_TXN_COMMON_PROPERTIES: ideally this should just be included
        // here in a flattened struct, but `flatten` doesn't work with
        // `deny_unknown_fields`: https://serde.rs/attr-flatten.html#struct-flattening
        pub max_fee: Fee,
        pub signature: Vec<TransactionSignatureElem>,
        pub nonce: TransactionNonce,

        pub sender_address: ContractAddress,
        pub calldata: Vec<CallParam>,
    }

    #[serde_as]
    #[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
    #[cfg_attr(any(test, feature = "rpc-full-serde"), derive(serde::Serialize))]
    #[serde(deny_unknown_fields)]
    pub struct BroadcastedInvokeTransactionV3 {
        pub version: TransactionVersion,
        pub signature: Vec<TransactionSignatureElem>,
        pub nonce: TransactionNonce,
        pub resource_bounds: super::ResourceBounds,
        #[serde_as(as = "pathfinder_serde::TipAsHexStr")]
        pub tip: Tip,
        pub paymaster_data: Vec<PaymasterDataElem>,
        pub account_deployment_data: Vec<AccountDeploymentDataElem>,
        pub nonce_data_availability_mode: super::DataAvailabilityMode,
        pub fee_data_availability_mode: super::DataAvailabilityMode,

        pub sender_address: ContractAddress,
        pub calldata: Vec<CallParam>,
    }

    impl BroadcastedTransaction {
        pub fn into_common(self, chain_id: ChainId) -> pathfinder_common::transaction::Transaction {
            use pathfinder_common::transaction::*;

            let variant = match self {
                BroadcastedTransaction::Declare(BroadcastedDeclareTransaction::V0(declare)) => {
                    let class_hash = declare.contract_class.class_hash().unwrap().hash();
                    TransactionVariant::DeclareV0(DeclareTransactionV0V1 {
                        class_hash,
                        max_fee: declare.max_fee,
                        nonce: Default::default(),
                        signature: declare.signature,
                        sender_address: declare.sender_address,
                    })
                }
                BroadcastedTransaction::Declare(BroadcastedDeclareTransaction::V1(declare)) => {
                    let class_hash = declare.contract_class.class_hash().unwrap().hash();
                    TransactionVariant::DeclareV1(DeclareTransactionV0V1 {
                        class_hash,
                        max_fee: declare.max_fee,
                        nonce: declare.nonce,
                        signature: declare.signature,
                        sender_address: declare.sender_address,
                    })
                }
                BroadcastedTransaction::Declare(BroadcastedDeclareTransaction::V2(declare)) => {
                    let class_hash = declare.contract_class.class_hash().unwrap().hash();
                    TransactionVariant::DeclareV2(DeclareTransactionV2 {
                        class_hash,
                        max_fee: declare.max_fee,
                        nonce: declare.nonce,
                        sender_address: declare.sender_address,
                        signature: declare.signature,
                        compiled_class_hash: declare.compiled_class_hash,
                    })
                }
                BroadcastedTransaction::Declare(BroadcastedDeclareTransaction::V3(declare)) => {
                    let class_hash = declare.contract_class.class_hash().unwrap().hash();
                    TransactionVariant::DeclareV3(DeclareTransactionV3 {
                        class_hash,
                        nonce: declare.nonce,
                        sender_address: declare.sender_address,
                        signature: declare.signature,
                        compiled_class_hash: declare.compiled_class_hash,
                        nonce_data_availability_mode: declare.nonce_data_availability_mode.into(),
                        fee_data_availability_mode: declare.fee_data_availability_mode.into(),
                        resource_bounds: declare.resource_bounds.into(),
                        tip: declare.tip,
                        paymaster_data: declare.paymaster_data,
                        account_deployment_data: declare.account_deployment_data,
                    })
                }
                BroadcastedTransaction::DeployAccount(
                    BroadcastedDeployAccountTransaction::V0V1(deploy),
                ) => TransactionVariant::DeployAccountV0V1(DeployAccountTransactionV0V1 {
                    contract_address: deploy.deployed_contract_address(),
                    max_fee: deploy.max_fee,
                    version: deploy.version,
                    signature: deploy.signature,
                    nonce: deploy.nonce,
                    contract_address_salt: deploy.contract_address_salt,
                    constructor_calldata: deploy.constructor_calldata,
                    class_hash: deploy.class_hash,
                }),
                BroadcastedTransaction::DeployAccount(BroadcastedDeployAccountTransaction::V3(
                    deploy,
                )) => TransactionVariant::DeployAccountV3(DeployAccountTransactionV3 {
                    class_hash: deploy.class_hash,
                    nonce: deploy.nonce,
                    contract_address: deploy.deployed_contract_address(),
                    contract_address_salt: deploy.contract_address_salt,
                    constructor_calldata: deploy.constructor_calldata,
                    signature: deploy.signature,
                    nonce_data_availability_mode: deploy.nonce_data_availability_mode.into(),
                    fee_data_availability_mode: deploy.fee_data_availability_mode.into(),
                    resource_bounds: deploy.resource_bounds.into(),
                    tip: deploy.tip,
                    paymaster_data: deploy.paymaster_data,
                }),
                BroadcastedTransaction::Invoke(BroadcastedInvokeTransaction::V0(invoke)) => {
                    TransactionVariant::InvokeV0(InvokeTransactionV0 {
                        calldata: invoke.calldata,
                        sender_address: invoke.contract_address,
                        entry_point_type: Some(EntryPointType::External),
                        entry_point_selector: invoke.entry_point_selector,
                        max_fee: invoke.max_fee,
                        signature: invoke.signature,
                    })
                }
                BroadcastedTransaction::Invoke(BroadcastedInvokeTransaction::V1(invoke)) => {
                    TransactionVariant::InvokeV1(InvokeTransactionV1 {
                        calldata: invoke.calldata,
                        sender_address: invoke.sender_address,
                        max_fee: invoke.max_fee,
                        signature: invoke.signature,
                        nonce: invoke.nonce,
                    })
                }
                BroadcastedTransaction::Invoke(BroadcastedInvokeTransaction::V3(invoke)) => {
                    TransactionVariant::InvokeV3(InvokeTransactionV3 {
                        nonce: invoke.nonce,
                        sender_address: invoke.sender_address,
                        signature: invoke.signature,
                        nonce_data_availability_mode: invoke.nonce_data_availability_mode.into(),
                        fee_data_availability_mode: invoke.fee_data_availability_mode.into(),
                        resource_bounds: invoke.resource_bounds.into(),
                        tip: invoke.tip,
                        paymaster_data: invoke.paymaster_data,
                        calldata: invoke.calldata,
                        account_deployment_data: invoke.account_deployment_data,
                    })
                }
            };

            let hash = variant.calculate_hash(chain_id);
            Transaction { hash, variant }
        }
    }

    #[cfg(test)]
    mod tests {
        macro_rules! fixture {
            ($file_name:literal) => {
                include_str!(concat!("../../fixtures/0.6.0/", $file_name)).replace(&[' ', '\n'], "")
            };
        }

        /// The aim of these tests is to check if deserialization works correctly
        /// **without resorting to serialization to prepare the test data**,
        /// which in itself could contain an "opposite phase" bug that cancels out.
        ///
        /// Serialization is tested btw, because the fixture and the data is already available.
        ///
        /// These tests were added due to recurring regressions stemming from, among others:
        /// - `serde(flatten)` and it's side-effects (for example when used in conjunction with `skip_serializing_none`),
        /// - `*AsDecimalStr*` creeping in from `sequencer::reply` as opposed to spec.
        mod serde {
            use super::super::*;
            use crate::v02::types::{
                CairoContractClass, ContractEntryPoints, DataAvailabilityMode, SierraContractClass,
                SierraEntryPoint, SierraEntryPoints,
            };
            use crate::v02::types::{ResourceBound, ResourceBounds};
            use pathfinder_common::{felt, ResourcePricePerUnit};
            use pathfinder_common::{macro_prelude::*, ResourceAmount};
            use pretty_assertions_sorted::assert_eq;

            #[test]
            fn broadcasted_transaction() {
                let contract_class = CairoContractClass {
                    program: "program".to_owned(),
                    entry_points_by_type: ContractEntryPoints {
                        constructor: vec![],
                        external: vec![],
                        l1_handler: vec![],
                    },
                    abi: None,
                };
                let txs = vec![
                    BroadcastedTransaction::Declare(BroadcastedDeclareTransaction::V1(
                        BroadcastedDeclareTransactionV1 {
                            max_fee: fee!("0x5"),
                            version: TransactionVersion::ONE,
                            signature: vec![transaction_signature_elem!("0x7")],
                            nonce: transaction_nonce!("0x8"),
                            contract_class,
                            sender_address: contract_address!("0xa"),
                        },
                    )),
                    BroadcastedTransaction::Declare(BroadcastedDeclareTransaction::V2(
                        BroadcastedDeclareTransactionV2 {
                            max_fee: fee!("0x51"),
                            version: TransactionVersion::TWO,
                            signature: vec![transaction_signature_elem!("0x71")],
                            nonce: transaction_nonce!("0x81"),
                            compiled_class_hash: casm_hash!("0x91"),
                            contract_class: SierraContractClass {
                                sierra_program: vec![felt!("0x4"), felt!("0x5")],
                                contract_class_version: "0.1.0".into(),
                                entry_points_by_type: SierraEntryPoints {
                                    constructor: vec![SierraEntryPoint {
                                        function_idx: 1,
                                        selector: felt!("0x1"),
                                    }],
                                    external: vec![SierraEntryPoint {
                                        function_idx: 2,
                                        selector: felt!("0x2"),
                                    }],
                                    l1_handler: vec![SierraEntryPoint {
                                        function_idx: 3,
                                        selector: felt!("0x3"),
                                    }],
                                },
                                abi: r#"[{"type":"function","name":"foo"}]"#.into(),
                            },
                            sender_address: contract_address!("0xa1"),
                        },
                    )),
                    BroadcastedTransaction::Declare(BroadcastedDeclareTransaction::V3(
                        BroadcastedDeclareTransactionV3 {
                            version: TransactionVersion::THREE,
                            signature: vec![transaction_signature_elem!("0x71")],
                            nonce: transaction_nonce!("0x81"),
                            resource_bounds: ResourceBounds {
                                l1_gas: ResourceBound {
                                    max_amount: ResourceAmount(0x1111),
                                    max_price_per_unit: ResourcePricePerUnit(0x2222),
                                },
                                l2_gas: ResourceBound {
                                    max_amount: ResourceAmount(0),
                                    max_price_per_unit: ResourcePricePerUnit(0),
                                },
                            },
                            tip: Tip(0x1234),
                            paymaster_data: vec![
                                paymaster_data_elem!("0x1"),
                                paymaster_data_elem!("0x2"),
                            ],
                            account_deployment_data: vec![
                                account_deployment_data_elem!("0x3"),
                                account_deployment_data_elem!("0x4"),
                            ],
                            nonce_data_availability_mode: DataAvailabilityMode::L1,
                            fee_data_availability_mode: DataAvailabilityMode::L2,
                            compiled_class_hash: casm_hash!("0x91"),
                            contract_class: SierraContractClass {
                                sierra_program: vec![felt!("0x4"), felt!("0x5")],
                                contract_class_version: "0.1.0".into(),
                                entry_points_by_type: SierraEntryPoints {
                                    constructor: vec![SierraEntryPoint {
                                        function_idx: 1,
                                        selector: felt!("0x1"),
                                    }],
                                    external: vec![SierraEntryPoint {
                                        function_idx: 2,
                                        selector: felt!("0x2"),
                                    }],
                                    l1_handler: vec![SierraEntryPoint {
                                        function_idx: 3,
                                        selector: felt!("0x3"),
                                    }],
                                },
                                abi: r#"[{"type":"function","name":"foo"}]"#.into(),
                            },
                            sender_address: contract_address!("0xa1"),
                        },
                    )),
                    BroadcastedTransaction::Invoke(BroadcastedInvokeTransaction::V1(
                        BroadcastedInvokeTransactionV1 {
                            version: TransactionVersion::ONE,
                            max_fee: fee!("0x6"),
                            signature: vec![transaction_signature_elem!("0x7")],
                            nonce: transaction_nonce!("0x8"),
                            sender_address: contract_address!("0xaaa"),
                            calldata: vec![call_param!("0xff")],
                        },
                    )),
                    BroadcastedTransaction::Invoke(BroadcastedInvokeTransaction::V1(
                        BroadcastedInvokeTransactionV1 {
                            version: TransactionVersion::ONE_WITH_QUERY_VERSION,
                            max_fee: fee!("0x6"),
                            signature: vec![transaction_signature_elem!("0x7")],
                            nonce: transaction_nonce!("0x8"),
                            sender_address: contract_address!("0xaaa"),
                            calldata: vec![call_param!("0xff")],
                        },
                    )),
                    BroadcastedTransaction::Invoke(BroadcastedInvokeTransaction::V3(
                        BroadcastedInvokeTransactionV3 {
                            version: TransactionVersion::THREE_WITH_QUERY_VERSION,
                            signature: vec![transaction_signature_elem!("0x7")],
                            nonce: transaction_nonce!("0x8"),
                            resource_bounds: ResourceBounds {
                                l1_gas: ResourceBound {
                                    max_amount: ResourceAmount(0x1111),
                                    max_price_per_unit: ResourcePricePerUnit(0x2222),
                                },
                                l2_gas: ResourceBound {
                                    max_amount: ResourceAmount(0),
                                    max_price_per_unit: ResourcePricePerUnit(0),
                                },
                            },
                            tip: Tip(0x1234),
                            paymaster_data: vec![
                                paymaster_data_elem!("0x1"),
                                paymaster_data_elem!("0x2"),
                            ],
                            account_deployment_data: vec![
                                account_deployment_data_elem!("0x3"),
                                account_deployment_data_elem!("0x4"),
                            ],
                            nonce_data_availability_mode: DataAvailabilityMode::L1,
                            fee_data_availability_mode: DataAvailabilityMode::L2,
                            sender_address: contract_address!("0xaaa"),
                            calldata: vec![call_param!("0xff")],
                        },
                    )),
                    BroadcastedTransaction::DeployAccount(BroadcastedDeployAccountTransaction::V3(
                        BroadcastedDeployAccountTransactionV3 {
                            version: TransactionVersion::THREE_WITH_QUERY_VERSION,
                            signature: vec![transaction_signature_elem!("0x7")],
                            nonce: transaction_nonce!("0x8"),
                            resource_bounds: ResourceBounds {
                                l1_gas: ResourceBound {
                                    max_amount: ResourceAmount(0x1111),
                                    max_price_per_unit: ResourcePricePerUnit(0x2222),
                                },
                                l2_gas: ResourceBound {
                                    max_amount: ResourceAmount(0),
                                    max_price_per_unit: ResourcePricePerUnit(0),
                                },
                            },
                            tip: Tip(0x1234),
                            paymaster_data: vec![
                                paymaster_data_elem!("0x1"),
                                paymaster_data_elem!("0x2"),
                            ],
                            nonce_data_availability_mode: DataAvailabilityMode::L1,
                            fee_data_availability_mode: DataAvailabilityMode::L2,
                            contract_address_salt: contract_address_salt!("0x99999"),
                            class_hash: class_hash!("0xddde"),
                            constructor_calldata: vec![call_param!("0xfe")],
                        },
                    )),
                ];

                let json_fixture = fixture!("broadcasted_transactions.json");

                assert_eq!(serde_json::to_string(&txs).unwrap(), json_fixture);
                assert_eq!(
                    serde_json::from_str::<Vec<BroadcastedTransaction>>(&json_fixture).unwrap(),
                    txs
                );
            }
        }
    }
}

/// Groups all strictly output types of the RPC API.
pub mod reply {
    use serde::Serialize;

    /// L2 Block status as returned by the RPC API.
    #[derive(Copy, Clone, Debug, Serialize, PartialEq, Eq)]
    #[cfg_attr(any(test, feature = "rpc-full-serde"), derive(serde::Deserialize))]
    #[serde(deny_unknown_fields)]
    pub enum BlockStatus {
        #[serde(rename = "PENDING")]
        Pending,
        #[serde(rename = "ACCEPTED_ON_L2")]
        AcceptedOnL2,
        #[serde(rename = "ACCEPTED_ON_L1")]
        AcceptedOnL1,
        #[serde(rename = "REJECTED")]
        Rejected,
    }

    impl BlockStatus {
        pub fn is_pending(&self) -> bool {
            self == &Self::Pending
        }
    }

    impl From<starknet_gateway_types::reply::Status> for BlockStatus {
        fn from(status: starknet_gateway_types::reply::Status) -> Self {
            use starknet_gateway_types::reply::Status::*;

            match status {
                // TODO verify this mapping with Starkware
                AcceptedOnL1 => BlockStatus::AcceptedOnL1,
                AcceptedOnL2 => BlockStatus::AcceptedOnL2,
                NotReceived => BlockStatus::Rejected,
                Pending => BlockStatus::Pending,
                Received => BlockStatus::Pending,
                Rejected => BlockStatus::Rejected,
                Reverted => BlockStatus::Rejected,
                Aborted => BlockStatus::Rejected,
            }
        }
    }
}
