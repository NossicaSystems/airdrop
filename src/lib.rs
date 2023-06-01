//! # A Concordium V1 smart contract
use concordium_cis2::*;
use concordium_std::*;
use sha256::digest;

use core::fmt::Debug;

/// Contract token ID type.
/// To save bytes we use a token ID type limited to a `u32`.
type ContractTokenId = TokenIdU32;

type ContractTokenAmount = TokenAmountU8;

/// The parameter for the contract function `mint` which mints a number of
/// tokens to a given address.
#[derive(Serial, Deserial, SchemaType)]
struct InitParams {
    whitelist: Vec<AccountAddress>,
    nft_limit: u32,
    nft_time_limit: u64,
    reserve: u32,
    base_url: String,
    selected_index: bool,
}

/// The parameter type for the contract function `contract_claim_nft`.
#[derive(Debug, Serialize, SchemaType)]
pub struct ClaimNFTParams {
    proof: Vec<String>,
    node: AccountAddress,
    selected_token: ContractTokenId,
}

#[derive(Serialize, SchemaType, PartialEq, Debug)]
struct ViewResult {
    claimed_tokens: HashMap<ContractTokenId, AccountAddress>,
}

#[derive(Serialize, SchemaType, PartialEq, Debug)]
struct CheckOwnerReply {
    address: Option<AccountAddress>,
}

//
#[derive(Serial, Deserial, SchemaType, Clone)]
pub struct MerkleTree {
    length: u8,
    hash_tree: Vec<String>,
    hashroot: String,
    steps: Vec<u8>,
}

/// Your smart contract state.
#[derive(Serial, Deserial, Clone)]
pub struct State {
    /// Next token ID.  Used if the user is just claiming tokens in sequential order.
    next_token_id: u32,
    // Set of taken indexes.  Used if the user is claiming specific indexes.
    taken_indexes: Option<HashMap<ContractTokenId, AccountAddress>>,
    // Max number of nfts that can be minted before hitting reserve
    nft_limit: u32,
    // Number of nfts which are held in reserve
    nft_reserve: Option<u32>,
    // Airdrop time limit
    nft_time_limit: Option<Timestamp>,
    // Whitelist proof
    merkle_tree: Option<MerkleTree>,
    // Base url for these NFTs
    base_url: String, // something like "https://some.example/token/";
}

impl State {
    /// Creates a new state with no tokens.
    fn empty() -> Self {
        State {
            next_token_id: 0,
            nft_limit: 1,
            merkle_tree: None,
            nft_time_limit: None,
            nft_reserve: None,
            base_url: String::new(),
            taken_indexes: None,
        }
    }

    // Basic merkle tree implementation
    // This will produce merkle trees like the following (note the real values would be hashed)
    // Example 1 - input 1,2,3
    //  1    2    3   3
    //    12        33
    //        1233
    //
    // Example 2 - input 1,2,3,4,5,6
    // 1   2   3   4   5    6
    //  12       34      56    56
    //      1234           5656
    //           12345656
    pub fn create_hash_tree(&mut self, nodes: Vec<String>) {
        let mut working_vec: Vec<String> = vec![];
        for node in nodes {
            working_vec.push(digest(node));
        }
        let mut working_node_total: usize = working_vec.len();
        let mut steps: Vec<u8> = Vec::new();

        if working_vec.len() % 2 == 1 {
            working_vec.push(working_vec[working_node_total - 1].clone());
            working_node_total += 1;
        }

        let initial_length = working_node_total;
        let mut startpoint = 0;
        let mut vec_to_add: Vec<String> = Vec::new();

        loop {
            // make sure tree is even
            if working_node_total % 2 == 1 {
                working_vec.push(working_vec.last().unwrap().clone());
            }

            for index in (startpoint..working_vec.len()).step_by(2) {
                vec_to_add.push(digest(working_vec[index].clone() + &working_vec[index + 1]));
            }

            startpoint = working_vec.len();
            working_vec.append(&mut vec_to_add.clone());
            working_node_total = working_vec.len();

            if (vec_to_add.len()) / 2 == 1 {
                steps.push((vec_to_add.len() + 1).try_into().unwrap());
            } else {
                steps.push((vec_to_add.len()).try_into().unwrap());
            }

            if vec_to_add.len() == 1 {
                self.merkle_tree = Some(MerkleTree {
                    length: initial_length as u8,
                    hashroot: working_vec.last().unwrap().clone(),
                    steps,
                    hash_tree: working_vec.clone(),
                });

                return;
            }
            vec_to_add.clear();
        }
    }

    // Use this to get the node chain for a given value.
    // Returns None if the value is not found.
    pub fn get_hash_proof(&self, test: String) -> Option<Vec<String>> {
        self.merkle_tree.as_ref()?;
        let local_tree = self.merkle_tree.as_ref().unwrap();

        let steps = &local_tree.steps;
        let mut end_point: usize = local_tree.length as usize;
        let nodes: &Vec<String> = &local_tree.hash_tree;
        let mut hunted: String = test;
        let mut startpoint: usize = 0;
        let mut step_number = 0;
        let mut proof: Vec<String> = Vec::new();
        let mut index = 0;
        while startpoint + index < end_point {
            if hunted == local_tree.hashroot {
                proof.push(hunted);
                return Some(proof);
            }

            if nodes[startpoint + index] == hunted {
                proof.push(hunted);
                if index % 2 == 1 {
                    // it is on the right hand side
                    hunted = digest(
                        nodes[startpoint + index - 1].clone() + &nodes[startpoint + index],
                    );
                } else {
                    // it is on the left hand side
                    hunted = digest(
                        nodes[startpoint + index].clone() + &nodes[startpoint + index + 1],
                    );
                }
                startpoint = end_point;
                end_point += steps[step_number] as usize;
                step_number += 1;
                index = 0;
                continue;
            }

            index += 1;
        }
        None
    }

    // Use this to compare the user's proof with our's
    pub fn check_proof(&self, test: &ClaimNFTParams) -> bool {
        let claimer = digest(
            test.node
                .0
                .iter()
                .map(|byte| format!("{:02X}", byte))
                .collect::<Vec<String>>()
                .concat(),
        );

        let master_proof = self.get_hash_proof(claimer).unwrap();
        master_proof == test.proof
    }

    // Checks to see whether a given value is in the tree
    // Generally used in testing
    pub fn check_hash_value(&self, test_address: String) -> bool {
        if self.merkle_tree.is_none() {
            return false;
        };
        let tree = self.merkle_tree.as_ref().unwrap();

        let steps = &tree.steps;
        let mut end_point = tree.length as usize;
        let nodes = &tree.hash_tree;
        let mut hunted = test_address;
        let mut startpoint = 0;
        let mut step_number = 0;

        let mut index: usize = 0;
        while startpoint + index < end_point {
            if hunted.eq(&tree.hashroot) {
                return true;
            }

            if nodes[startpoint + index] == hunted {
                if index % 2 == 1 {
                    // it is on the right hand side
                    hunted = digest(
                        nodes[startpoint + index - 1].clone() + &nodes[startpoint + index],
                    );
                } else {
                    // it is on the left hand side
                    hunted = digest(
                        nodes[startpoint + index].clone() + &nodes[startpoint + index + 1],
                    );
                }
                startpoint = end_point;
                end_point += steps[step_number] as usize;
                step_number +=  1;
                index = 0;
                continue;
            }

            index +=  1;
        }
        false
    }
}

/// Your smart contract errors.
#[derive(Debug, PartialEq, Eq, Reject, Serial, SchemaType)]
enum Error {
    /// Failed parsing the parameter.
    #[from(ParseError)]
    NFTLimitReached,
    AddressNotOnWhitelist,
    AirdropNowClosed,
    MintingLogMalformed,
    MintingLogFull,
    MetaDataLogMalformed,
    MetaDataLogFull,
    IndexAlreadyClaimed,
}

/// Init function that creates a new smart contract.
#[init(contract = "airdrop_project", parameter = "InitParams")]
fn init<S: HasStateApi>(
    ctx: &impl HasInitContext,
    _state_builder: &mut StateBuilder<S>,
) -> InitResult<State> {
    let params: InitParams = ctx.parameter_cursor().get()?;
    let mut state: State = State::empty();
    state.nft_limit = params.nft_limit;
    state.base_url = params.base_url;

    if params.nft_time_limit != 0 {
        state.nft_time_limit = Some(Timestamp::from_timestamp_millis(params.nft_time_limit));
    }

    if params.reserve != 0 {
        state.nft_reserve = Some(params.reserve);
    }

    if params.selected_index {
        state.taken_indexes = Some(HashMap::default());
    }

    if !params.whitelist.is_empty() {
        let mut whitelist: Vec<String> = vec![];
        for address in params.whitelist {
            whitelist.push(
                address
                    .0
                    .iter()
                    .map(|byte| format!("{:02X}", byte))
                    .collect::<Vec<String>>()
                    .concat(),
            );
        }
        state.create_hash_tree(whitelist);
    }

    Ok(state)
}

/// Claims an NFT
#[receive(
    contract = "airdrop_project",
    name = "claim_nft",
    parameter = "ClaimNFTParams",
    error = "Error",
    mutable,
    enable_logger
)]
fn claim_nft<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State, StateApiType = S>,
    logger: &mut impl HasLogger,
) -> Result<(), Error> {
    let state = host.state_mut();

    if let Some(time_limit) = state.nft_time_limit {
        if time_limit > Timestamp::from_timestamp_millis(0) && ctx.metadata().slot_time() > state.nft_time_limit.unwrap() {
            return Err(Error::AirdropNowClosed);
        }                
    }

    let params: ClaimNFTParams = ctx.parameter_cursor().get()?;
    let current_token_id = state.next_token_id;
    if current_token_id == state.nft_limit {
        return Err(Error::NFTLimitReached);
    }

    let claimer = params.node;

    // if there is a whitelist and no reserve only whitelist can by
    // if there is no whitelist everyone can buy
    // if there is a reserve and a whitelist only whitelist can by reserve
    if ((state.merkle_tree.is_some() && state.nft_reserve.is_none()) 
        || (state.merkle_tree.is_some() && state.next_token_id >= (state.nft_limit - state.nft_reserve.unwrap_or(0)))) 
        && (params.proof.is_empty() || !state.check_proof(&params)) {
        return Err(Error::AddressNotOnWhitelist);
    }

    // This is where the code differentiates between the user claiming the next available token
    // and the user claiming a specific one they have requested.
    let token_id_to_use = if state.taken_indexes.is_some() {
        if state
            .taken_indexes
            .as_ref()
            .unwrap()
            .contains_key(&params.selected_token)
        {
            return Err(Error::IndexAlreadyClaimed);
        }
        params.selected_token
    } else {
        ContractTokenId::from(current_token_id)
    };

    // Event for minted token.
    let log_mint_result = logger.log(&Cis2Event::Mint(MintEvent {
        token_id: token_id_to_use,
        amount: ContractTokenAmount::from(1),
        owner: concordium_std::Address::Account(claimer),
    }));

    match log_mint_result {
        Ok(_) => (),
        Err(error) => match error {
            LogError::Full => {
                return Err(Error::MintingLogFull);
            }
            LogError::Malformed => {
                return Err(Error::MintingLogMalformed);
            }
        },
    }

    let url: String = state.base_url.clone() + &token_id_to_use.to_string();

    // Metadata URL for the token.
    let log_meta_result = logger.log(&Cis2Event::TokenMetadata::<_, ContractTokenAmount>(
        TokenMetadataEvent {
            token_id: token_id_to_use,
            metadata_url: MetadataUrl {
                url,
                hash: None,
            },
        },
    ));

    match log_meta_result {
        Ok(_) => (),
        Err(error) => match error {
            LogError::Full => {
                return Err(Error::MetaDataLogFull);
            }
            LogError::Malformed => {
                return Err(Error::MetaDataLogMalformed);
            }
        },
    }

    if state.taken_indexes.is_some() {
        state
            .taken_indexes
            .as_mut()
            .unwrap()
            .insert(token_id_to_use, claimer);
    } else {
        state.next_token_id += 1;
    }

    Ok(())
}

/// View function that returns the content of the state.
#[receive(
    contract = "airdrop_project",
    name = "view",
    return_value = "ViewResult"
)]
fn view<S: HasStateApi>(
    _ctx: &impl HasReceiveContext,
    host: &impl HasHost<State, StateApiType = S>,
) -> ReceiveResult<ViewResult> {
    Ok(ViewResult {
        claimed_tokens: host
            .state()
            .taken_indexes
            .clone()
            .unwrap_or(HashMap::default()),
    })
}

/// View function that returns the total supply of available NFTs
#[receive(
    contract = "airdrop_project",
    name = "total_supply",
    return_value = "u32"
)]
fn total_supply<S: HasStateApi>(
    _ctx: &impl HasReceiveContext,
    host: &impl HasHost<State, StateApiType = S>,) -> ReceiveResult<u32> {
    Ok(host.state().nft_limit)
}

/// View function that returns the current supply of available NFTs
#[receive(
    contract = "airdrop_project",
    name = "current_supply",
    return_value = "u32"
)]
fn current_supply<S: HasStateApi>(
    _ctx: &impl HasReceiveContext,
    host: &impl HasHost<State, StateApiType = S>,
) -> ReceiveResult<u32> {
    let current_claimed = if host.state().taken_indexes.is_some() {
        host.state().taken_indexes.as_ref().unwrap().len() as u32
    } else {
        host.state().next_token_id
    };

    Ok(host.state().nft_limit - current_claimed)
}

/// View function that returns the owner of tokens or None if no one owns it
#[receive(
    contract = "airdrop_project",
    name = "check_owner",
    parameter = "ContractTokenID",
    return_value = "CheckOwnerReply"
)]
fn check_owner<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &impl HasHost<State, StateApiType = S>,
) -> ReceiveResult<CheckOwnerReply> {
    let params: ContractTokenId = ctx.parameter_cursor().get()?;

    if host.state().taken_indexes.as_ref().is_none() {
        return Ok(CheckOwnerReply { address: None });
    }

    if !host
        .state()
        .taken_indexes
        .as_ref()
        .unwrap()
        .contains_key(&params)
    {
        return Ok(CheckOwnerReply { address: None });
    }

    return Ok(CheckOwnerReply {
        address: Some(
            *host
                .state()
                .taken_indexes
                .as_ref()
                .unwrap()
                .get(&params)
                .unwrap(),
        ),
    });
}

#[concordium_cfg_test]
mod tests {
    use super::*;
    use test_infrastructure::*;

    #[concordium_test]
    /// Test that initializing the contract succeeds with some state.
    fn test_init() {
        let mut ctx = TestInitContext::empty();
        ctx.set_metadata_slot_time(Timestamp::from_timestamp_millis(1));
        let mut state_builder = TestStateBuilder::new();

        let params = InitParams {
            nft_limit: 0,
            nft_time_limit: 0,
            whitelist: vec![],
            reserve: 0,
            base_url: String::new(),
            selected_index: false,
        };

        let parameter_bytes = to_bytes(&params);
        ctx.set_parameter(&parameter_bytes);

        let state_result = init(&ctx, &mut state_builder);
        state_result.unwrap();
    }

    #[concordium_test]
    fn test_mint_no_reserve_no_whitelist() {
        let mut ctx = TestInitContext::empty();
        ctx.set_metadata_slot_time(Timestamp::from_timestamp_millis(1));
        let mut state_builder = TestStateBuilder::new();

        const ACCOUNT_0: AccountAddress = AccountAddress([0u8; 32]);
        // This should allow anyone to purchase 1 NFT
        let params = InitParams {
            nft_limit: 1,
            nft_time_limit: 0,
            whitelist: vec![],
            reserve: 0,
            base_url: String::new(),
            selected_index: false,
        };

        let parameter_bytes = to_bytes(&params);
        ctx.set_parameter(&parameter_bytes);

        let state_result = init(&ctx, &mut state_builder);
        let new_state = state_result.unwrap();
        assert_eq!(new_state.nft_limit, 1);

        let mut ctx_claim = TestReceiveContext::empty();
        ctx_claim.set_metadata_slot_time(Timestamp::from_timestamp_millis(1));
        let mint_params = ClaimNFTParams {
            node: ACCOUNT_0,
            proof: vec![],
            selected_token: concordium_cis2::TokenIdU32(0),
        };

        let mut host = TestHost::new(new_state, state_builder);

        let claim_parameter_bytes = to_bytes(&mint_params);
        ctx_claim.set_parameter(&claim_parameter_bytes);
        let mut logger = TestLogger::init();

        let claim_result = claim_nft(&ctx_claim, &mut host, &mut logger);
        assert_eq!(claim_result.is_ok(), true);

        let claim_result_bad: Result<(), Error> = claim_nft(&ctx_claim, &mut host, &mut logger);
        assert_eq!(claim_result_bad, Err(Error::NFTLimitReached));
    }

    #[concordium_test]
    fn test_whitelist() {
        let mut ctx = TestInitContext::empty();
        ctx.set_metadata_slot_time(Timestamp::from_timestamp_millis(1));
        let mut state_builder = TestStateBuilder::new();

        const ACCOUNT_0: AccountAddress = AccountAddress([0u8; 32]);
        const ACCOUNT_1: AccountAddress = AccountAddress([1u8; 32]);
        const ACCOUNT_2: AccountAddress = AccountAddress([2u8; 32]);
        const ACCOUNT_3: AccountAddress = AccountAddress([3u8; 32]);

        let whitelist: Vec<AccountAddress> = vec![ACCOUNT_0, ACCOUNT_1, ACCOUNT_2];

        // This should allow anyone to purchase 1 NFT
        let params = InitParams {
            nft_limit: 1,
            nft_time_limit: 0,
            whitelist: whitelist.clone(),
            reserve: 0,
            base_url: String::new(),
            selected_index: false,
        };

        let parameter_bytes = to_bytes(&params);
        ctx.set_parameter(&parameter_bytes);

        let state = init(&ctx, &mut state_builder).unwrap();

        // convert the addresses to strings
        let mut hashes: Vec<String> = vec![];
        for address in whitelist {
            hashes.push(digest(
                address
                    .0
                    .iter()
                    .map(|byte| format!("{:02X}", byte))
                    .collect::<Vec<String>>()
                    .concat(),
            ));
        }

        let bad_address = ACCOUNT_3
            .0
            .iter()
            .map(|byte| format!("{:02X}", byte))
            .collect::<Vec<String>>()
            .concat();

        assert_eq!(state.check_hash_value(hashes[0].clone()), true);
        assert_eq!(state.check_hash_value(hashes[1].clone()), true);
        assert_eq!(state.check_hash_value(hashes[2].clone()), true);
        assert_eq!(state.check_hash_value(bad_address), false);

        let a = digest(hashes[0].clone() + &hashes[1]);
        let b = digest(hashes[2].clone() + &hashes[2]); // MT will duplicated 4th element from 3rd
        let c = digest(a.clone() + &b);

        let test_merkle_proof = vec![hashes[0].clone(), a, c];

        let test_address = digest(
            ACCOUNT_0
                .0
                .iter()
                .map(|byte| format!("{:02X}", byte))
                .collect::<Vec<String>>()
                .concat(),
        );
        let merkle_proof = state.get_hash_proof(test_address).unwrap();
        assert_eq!(merkle_proof, test_merkle_proof);
    }

    #[concordium_test]
    fn test_merkle_proof() {
        let mut ctx = TestInitContext::empty();
        ctx.set_metadata_slot_time(Timestamp::from_timestamp_millis(1));
        let mut state_builder = TestStateBuilder::new();

        const ACCOUNT_0: AccountAddress = AccountAddress([0u8; 32]);
        const ACCOUNT_1: AccountAddress = AccountAddress([1u8; 32]);
        const ACCOUNT_2: AccountAddress = AccountAddress([2u8; 32]);
        const ACCOUNT_3: AccountAddress = AccountAddress([3u8; 32]);

        let whitelist: Vec<AccountAddress> = vec![ACCOUNT_0, ACCOUNT_1, ACCOUNT_2];

        // This should allow anyone to purchase 1 NFT
        let params = InitParams {
            nft_limit: 1,
            nft_time_limit: 0,
            whitelist: whitelist.clone(),
            reserve: 0,
            base_url: String::new(),
            selected_index: false,
        };

        let parameter_bytes = to_bytes(&params);
        ctx.set_parameter(&parameter_bytes);

        let state = init(&ctx, &mut state_builder).unwrap();

        // convert the addresses to strings
        let mut hashes: Vec<String> = vec![];
        for address in whitelist {
            hashes.push(digest(
                address
                    .0
                    .iter()
                    .map(|byte| format!("{:02X}", byte))
                    .collect::<Vec<String>>()
                    .concat(),
            ));
        }

        let bad_address = ACCOUNT_3
            .0
            .iter()
            .map(|byte| format!("{:02X}", byte))
            .collect::<Vec<String>>()
            .concat();

        assert_eq!(state.check_hash_value(hashes[0].clone()), true);
        assert_eq!(state.check_hash_value(hashes[1].clone()), true);
        assert_eq!(state.check_hash_value(hashes[2].clone()), true);
        assert_eq!(state.check_hash_value(bad_address), false);

        let a = digest(hashes[0].clone() + &hashes[1]);
        let b = digest(hashes[2].clone() + &hashes[2]); // MT will duplicated 4th element from 3rd
        let c = digest(a.clone() + &b);

        let test_merkle_proof = vec![hashes[0].clone(), a, c];

        let test_address = digest(
            ACCOUNT_0
                .0
                .iter()
                .map(|byte| format!("{:02X}", byte))
                .collect::<Vec<String>>()
                .concat(),
        );
        let merkle_proof = state.get_hash_proof(test_address).unwrap();
        assert_eq!(merkle_proof, test_merkle_proof);

        let proof_params = ClaimNFTParams {
            proof: test_merkle_proof.clone(),
            node: ACCOUNT_0,
            selected_token: concordium_cis2::TokenIdU32(0),
        };
        assert_eq!(state.check_proof(&proof_params), true);

        let proof_params = ClaimNFTParams {
            proof: test_merkle_proof.clone(),
            node: ACCOUNT_1,
            selected_token: concordium_cis2::TokenIdU32(0),
        };
        assert_eq!(state.check_proof(&proof_params), false);
    }

    #[concordium_test]
    fn test_claim_with_whitelist_full_reserve() {
        let mut ctx = TestInitContext::empty();
        let mut state_builder = TestStateBuilder::new();
        ctx.set_metadata_slot_time(Timestamp::from_timestamp_millis(1));

        const ACCOUNT_0: AccountAddress = AccountAddress([0u8; 32]);
        const ACCOUNT_1: AccountAddress = AccountAddress([1u8; 32]);

        let whitelist: Vec<AccountAddress> = vec![ACCOUNT_0, ACCOUNT_1];

        // This should allow anyone to purchase 1 NFT
        let params = InitParams {
            nft_limit: 4,
            nft_time_limit: 0,
            whitelist: whitelist.clone(),
            reserve: 4,
            base_url: String::new(),
            selected_index: false,
        };

        let parameter_bytes = to_bytes(&params);
        ctx.set_parameter(&parameter_bytes);

        let state = init(&ctx, &mut state_builder).unwrap();

        let mut test_proof: Vec<String> = vec![];
        let acc1 = digest(
            ACCOUNT_0
                .0
                .iter()
                .map(|byte| format!("{:02X}", byte))
                .collect::<Vec<String>>()
                .concat(),
        );
        let acc2 = digest(
            ACCOUNT_1
                .0
                .iter()
                .map(|byte| format!("{:02X}", byte))
                .collect::<Vec<String>>()
                .concat(),
        );

        test_proof.push(acc1.clone());
        test_proof.push(digest(acc1.clone() + &acc2));

        let mut ctx_claim = TestReceiveContext::empty();
        ctx_claim.set_metadata_slot_time(Timestamp::from_timestamp_millis(1));

        let mint_params = ClaimNFTParams {
            node: ACCOUNT_0,
            proof: test_proof.clone(),
            selected_token: concordium_cis2::TokenIdU32(0),
        };

        let mut host = TestHost::new(state, state_builder);

        let claim_parameter_bytes = to_bytes(&mint_params);
        ctx_claim.set_parameter(&claim_parameter_bytes);
        let mut logger = TestLogger::init();

        claim_nft(&ctx_claim, &mut host, &mut logger).unwrap();

        let mut ctx_bad_claim = TestReceiveContext::empty();
        ctx_bad_claim.set_metadata_slot_time(Timestamp::from_timestamp_millis(1));
        let mint_bad_params = ClaimNFTParams {
            node: ACCOUNT_1,
            proof: test_proof.clone(),
            selected_token: concordium_cis2::TokenIdU32(0),
        };
        let bad_claim_parameter_bytes = to_bytes(&mint_bad_params);
        ctx_bad_claim.set_parameter(&bad_claim_parameter_bytes);

        let claim_result_bad = claim_nft(&ctx_bad_claim, &mut host, &mut logger);
        claim_eq!(
            claim_result_bad,
            Err(Error::AddressNotOnWhitelist),
            "Function should fail with NFT error"
        );
    }

    #[concordium_test]
    fn test_claim_with_whitelist_no_reserve() {
        let mut ctx = TestInitContext::empty();
        let mut state_builder = TestStateBuilder::new();
        ctx.set_metadata_slot_time(Timestamp::from_timestamp_millis(1));
        const ACCOUNT_0: AccountAddress = AccountAddress([0u8; 32]);
        const ACCOUNT_1: AccountAddress = AccountAddress([1u8; 32]);

        let whitelist: Vec<AccountAddress> = vec![ACCOUNT_0, ACCOUNT_1];

        // This should allow anyone to purchase 1 NFT
        let params = InitParams {
            nft_limit: 4,
            nft_time_limit: 0,
            whitelist: whitelist.clone(),
            reserve: 0,
            base_url: String::new(),
            selected_index: false,
        };

        let mut test_proof: Vec<String> = vec![];
        let acc1 = digest(
            ACCOUNT_0
                .0
                .iter()
                .map(|byte| format!("{:02X}", byte))
                .collect::<Vec<String>>()
                .concat(),
        );
        let acc2 = digest(
            ACCOUNT_1
                .0
                .iter()
                .map(|byte| format!("{:02X}", byte))
                .collect::<Vec<String>>()
                .concat(),
        );

        test_proof.push(acc1.clone());
        test_proof.push(digest(acc1.clone() + &acc2));

        let parameter_bytes = to_bytes(&params);
        ctx.set_parameter(&parameter_bytes);

        let state = init(&ctx, &mut state_builder).unwrap();

        let mut ctx_claim = TestReceiveContext::empty();
        let mint_params = ClaimNFTParams {
            node: ACCOUNT_0,
            proof: test_proof.clone(),
            selected_token: concordium_cis2::TokenIdU32(0),
        };

        let mut host = TestHost::new(state, state_builder);

        let claim_parameter_bytes = to_bytes(&mint_params);
        ctx_claim.set_parameter(&claim_parameter_bytes);
        ctx_claim.set_metadata_slot_time(Timestamp::from_timestamp_millis(1));
        let mut logger = TestLogger::init();

        claim_nft(&ctx_claim, &mut host, &mut logger).unwrap();

        let mut ctx_bad_claim = TestReceiveContext::empty();
        ctx_bad_claim.set_metadata_slot_time(Timestamp::from_timestamp_millis(1));
        let mint_bad_params = ClaimNFTParams {
            node: ACCOUNT_1,
            proof: test_proof.clone(),
            selected_token: concordium_cis2::TokenIdU32(0),
        };
        let bad_claim_parameter_bytes = to_bytes(&mint_bad_params);
        ctx_bad_claim.set_parameter(&bad_claim_parameter_bytes);

        let claim_result_bad = claim_nft(&ctx_bad_claim, &mut host, &mut logger);
        claim_eq!(
            claim_result_bad,
            Err(Error::AddressNotOnWhitelist),
            "Function should fail with NFT error"
        );
    }

    #[concordium_test]
    fn test_claim_with_whitelist_partial_reserve() {
        let mut ctx = TestInitContext::empty();
        ctx.set_metadata_slot_time(Timestamp::from_timestamp_millis(1));
        let mut state_builder = TestStateBuilder::new();

        const ACCOUNT_0: AccountAddress = AccountAddress([0u8; 32]);
        const ACCOUNT_1: AccountAddress = AccountAddress([1u8; 32]);

        let whitelist: Vec<AccountAddress> = vec![ACCOUNT_0];

        // This should allow anyone to purchase 1 NFT
        let params = InitParams {
            nft_limit: 3,
            nft_time_limit: 0,
            whitelist: whitelist.clone(),
            reserve: 2,
            base_url: String::new(),
            selected_index: false,
        };

        let parameter_bytes = to_bytes(&params);
        ctx.set_parameter(&parameter_bytes);

        let state = init(&ctx, &mut state_builder).unwrap();

        let mut ctx_claim = TestReceiveContext::empty();
        ctx_claim.set_metadata_slot_time(Timestamp::from_timestamp_millis(1));

        let mint_params = ClaimNFTParams {
            node: ACCOUNT_1,
            proof: vec![],
            selected_token: concordium_cis2::TokenIdU32(0),
        };

        let mut host = TestHost::new(state, state_builder);

        let claim_parameter_bytes = to_bytes(&mint_params);
        ctx_claim.set_parameter(&claim_parameter_bytes);

        let mut logger = TestLogger::init();
        // this should not check the whitelist
        claim_nft(&ctx_claim, &mut host, &mut logger).unwrap();

        let mut ctx_wl_claim = TestReceiveContext::empty();
        ctx_wl_claim.set_metadata_slot_time(Timestamp::from_timestamp_millis(1));
        let address_hashed = digest(
            ACCOUNT_0
                .0
                .iter()
                .map(|byte| format!("{:02X}", byte))
                .collect::<Vec<String>>()
                .concat(),
        );

        let mut test_proof = vec![];

        test_proof.push(address_hashed.clone());
        test_proof.push(digest(address_hashed.clone() + &address_hashed));

        let mint_wl_params = ClaimNFTParams {
            node: ACCOUNT_0,
            proof: test_proof.clone(),
            selected_token: concordium_cis2::TokenIdU32(0),
        };

        let wl_claim_parameter_bytes = to_bytes(&mint_wl_params);
        ctx_wl_claim.set_parameter(&wl_claim_parameter_bytes);

        let mut logger = TestLogger::init();
        // this should check the whitelist and pass
        claim_nft(&ctx_wl_claim, &mut host, &mut logger).unwrap();

        // this should not check the whitelist and fail
        let fail_claim = claim_nft(&ctx_claim, &mut host, &mut logger);

        claim_eq!(
            fail_claim,
            Err(Error::AddressNotOnWhitelist),
            "Function should fail with whitelist error"
        );
    }

    #[concordium_test]
    fn test_mint_too_late() {
        let mut ctx = TestInitContext::empty();
        let mut state_builder = TestStateBuilder::new();
        ctx.set_metadata_slot_time(Timestamp::from_timestamp_millis(1));

        const ACCOUNT_0: AccountAddress = AccountAddress([0u8; 32]);

        // This should allow anyone to purchase 1 NFT
        let params = InitParams {
            nft_limit: 1,
            nft_time_limit: 10,
            whitelist: vec![],
            reserve: 0,
            base_url: String::new(),
            selected_index: false,
        };

        let parameter_bytes = to_bytes(&params);
        ctx.set_parameter(&parameter_bytes);

        let state_result = init(&ctx, &mut state_builder);
        let new_state = state_result.unwrap();
        assert_eq!(new_state.nft_limit, 1);

        let mut ctx_claim = TestReceiveContext::empty();
        ctx_claim.set_metadata_slot_time(Timestamp::from_timestamp_millis(1));
        let mint_params = ClaimNFTParams {
            node: ACCOUNT_0,
            proof: vec![],
            selected_token: concordium_cis2::TokenIdU32(0),
        };

        let mut host = TestHost::new(new_state, state_builder);

        let claim_parameter_bytes = to_bytes(&mint_params);
        ctx_claim.set_metadata_slot_time(Timestamp::from_timestamp_millis(11));
        ctx_claim.set_parameter(&claim_parameter_bytes);

        let mut logger = TestLogger::init();
        let claim_result = claim_nft(&ctx_claim, &mut host, &mut logger);
        claim_eq!(
            claim_result,
            Err(Error::AirdropNowClosed),
            "Function should fail with Airdrop closed error"
        );
    }

    #[concordium_test]
    fn test_mint_no_reserve_no_whitelist_selected_index() {
        let mut ctx = TestInitContext::empty();
        ctx.set_metadata_slot_time(Timestamp::from_timestamp_millis(1));
        let mut state_builder = TestStateBuilder::new();

        const ACCOUNT_0: AccountAddress = AccountAddress([0u8; 32]);
        // This should allow anyone to purchase 2 NFTs
        let params = InitParams {
            nft_limit: 2,
            nft_time_limit: 0,
            whitelist: vec![],
            reserve: 0,
            base_url: "https://some.example/token/".to_string(),
            selected_index: true,
        };

        let parameter_bytes = to_bytes(&params);
        ctx.set_parameter(&parameter_bytes);

        let state_result = init(&ctx, &mut state_builder);
        let new_state = state_result.unwrap();
        assert_eq!(new_state.nft_limit, 2);

        let mut ctx_claim = TestReceiveContext::empty();
        ctx_claim.set_metadata_slot_time(Timestamp::from_timestamp_millis(1));
        let mint_params = ClaimNFTParams {
            node: ACCOUNT_0,
            proof: vec![],
            selected_token: concordium_cis2::TokenIdU32(2),
        };

        let mut host = TestHost::new(new_state, state_builder);

        let claim_parameter_bytes = to_bytes(&mint_params);
        ctx_claim.set_parameter(&claim_parameter_bytes);
        let mut logger = TestLogger::init();

        let claim_result = claim_nft(&ctx_claim, &mut host, &mut logger);
        assert_eq!(claim_result.is_ok(), true);

        claim!(
            logger.logs.contains(&to_bytes(&Cis2Event::Mint(MintEvent {
                owner: concordium_std::Address::Account(ACCOUNT_0),
                token_id: ContractTokenId::from(2),
                amount: ContractTokenAmount::from(1),
            }))),
            "Expected an event for minting token 2"
        );

        claim!(
            logger.logs.contains(&to_bytes(
                &Cis2Event::TokenMetadata::<_, ContractTokenAmount>(TokenMetadataEvent {
                    token_id: concordium_cis2::TokenIdU32(2),
                    metadata_url: MetadataUrl {
                        url: "https://some.example/token/02000000".to_string(),
                        hash: None,
                    },
                })
            )),
            "Expected an event for token metadata for token 2"
        );

        // check that the token has the correct owner:
        let mut owner_ctx = TestReceiveContext::empty();
        let owner_params = concordium_cis2::TokenIdU32(2);
        let owner_parameter_bytes = to_bytes(&owner_params);
        owner_ctx.set_parameter(&owner_parameter_bytes);
        let owner_result: Result<CheckOwnerReply, Reject> = check_owner(&owner_ctx, &mut host);
        assert!(owner_result.is_ok());
        let result_details = owner_result.unwrap();
        assert!(result_details.address.is_some());
        assert_eq!(result_details.address.unwrap(), ACCOUNT_0);

        // check that the wrong token has the no owner:
        let mut non_owner_ctx = TestReceiveContext::empty();
        let non_owner_params = concordium_cis2::TokenIdU32(5);
        let non_owner_parameter_bytes = to_bytes(&non_owner_params);
        non_owner_ctx.set_parameter(&non_owner_parameter_bytes);
        let non_owner_result: Result<CheckOwnerReply, Reject> =
            check_owner(&non_owner_ctx, &mut host);
        assert!(non_owner_result.is_ok());
        let non_result_details = non_owner_result.unwrap();
        assert!(non_result_details.address.is_none());

        // Check the right amount of tokens exist and have been claimed
        assert_eq!(total_supply(&non_owner_ctx, &mut host).unwrap(), 2);
        assert_eq!(current_supply(&non_owner_ctx, &mut host).unwrap(), 1);

        let mut vr = HashMap::default();
        vr.insert(concordium_cis2::TokenIdU32(2), ACCOUNT_0);
        assert_eq!(
            view(&ctx_claim, &host).unwrap(),
            ViewResult { claimed_tokens: vr }
        );

        let claim_result_bad: Result<(), Error> = claim_nft(&ctx_claim, &mut host, &mut logger);
        assert_eq!(claim_result_bad, Err(Error::IndexAlreadyClaimed));
    }
}
