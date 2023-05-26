//! # A Concordium V1 smart contract
use concordium_std::*;
use concordium_cis2::*;
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
    whitelist:       Vec<AccountAddress>,
    nft_limit:       u32,
    nft_time_limit:  u64,
    reserve:         u32,
    base_url:        String,
}

#[derive(Serialize, SchemaType)]
struct ViewState {
    amount_of_claimed_tokens: u32,
}


#[derive(Serialize, SchemaType, PartialEq, Debug)]
struct ClaimReply {
    url: String,
}

/// The parameter type for the contract function `contract_claim_nft`.
#[derive(Debug, Serialize, SchemaType)]
pub struct ClaimNFTParams {
    proof:  Vec<String>,
    node:   AccountAddress,
}

// 
#[derive(Serial, Deserial, SchemaType, Clone)]
pub struct MerkleTree {
    length: u8,
    hash_tree: Vec<String>,
    hashroot: String,
    steps: Vec<u8>
}

impl MerkleTree {
    fn new() -> Self {
        MerkleTree {
            length: 0,
            hash_tree: Vec::new(),
            hashroot: String::new(),
            steps: Vec::new(),
        }
    }
}

/// Your smart contract state.
#[derive(Serial, Deserial, Clone)]
pub struct State {
    /// Next token ID
    next_token_id:          u32,
    // Max number of nfts that can be minted before hitting reserve
    nft_limit:              u32,        // todo: change to option
    // Number of nfts which are held in reserve
    nft_reserve:            u32,        // todo: change to option
    // Airdrop time limit
    nft_time_limit:         Option<Timestamp>,
    // Whitelist proof
    whitelist:              bool,       // todo: change to option on the merkle tree
    merkle_tree:            MerkleTree,
    // Base url for these NFTs
    base_url:               String      // something like "https://some.example/token/";
}

impl State {
    /// Creates a new state with no tokens.
    fn empty() -> Self {
        State {
            next_token_id: 0,
            nft_limit:     1,
            merkle_tree:    MerkleTree::new(),
            whitelist:      false,
            nft_time_limit: None,
            nft_reserve: 0,
            base_url: String::new(),
        }
    }

    // basic merkle tree implementation
    pub fn create_hash_tree(&mut self, nodes:  Vec<String>) {
        let mut working_vec: Vec<String> = vec!();
        for node in nodes {
            working_vec.push(digest(node));
        }
        let mut working_node_total: usize = working_vec.len();
        let mut steps: Vec<u8> = Vec::new();
        
        if working_vec.len() % 2 == 1 {
            working_vec.push(working_vec[working_node_total-1].clone());
            working_node_total+=1;
        }

        let initial_length = working_node_total;
        let mut startpoint = 0;
        let mut vec_to_add: Vec<String> = Vec::new();

        loop {
            // make sure tree is even
            if working_node_total % 2 == 1 {
                working_vec.push(working_vec.last().unwrap().clone());
            }
            
            for index in (startpoint .. working_vec.len()).step_by(2) {
                vec_to_add.push(digest(working_vec[index].clone() + &working_vec[index+1])); 
            }

            startpoint = working_vec.len();
            working_vec.append(&mut vec_to_add.clone());
            working_node_total = working_vec.len();

            if (vec_to_add.len()) / 2 == 1 {
                steps.push((vec_to_add.len()+1).try_into().unwrap());
            }
            else {
                steps.push((vec_to_add.len()).try_into().unwrap());
            }

            if vec_to_add.len() == 1 {
                self.merkle_tree = MerkleTree {
                    length: initial_length as u8,
                    hashroot: working_vec.last().unwrap().clone(),
                    steps: steps,
                    hash_tree: working_vec.clone(),
                };
               
                return;
            }
            vec_to_add.clear();
        }
    }
 
    // Use this to get the node chain for a given value.
    // Returns None if the value is not found.
    pub fn get_hash_proof(&self, test :String) -> Option<Vec<String>> {
        let steps = &self.merkle_tree.steps;
        let mut end_point: usize = self.merkle_tree.length as usize;
        let nodes: &Vec<String> = &self.merkle_tree.hash_tree;
        let mut hunted: String = test;
        let mut startpoint: usize = 0;
        let mut step_number = 0;
        let mut proof: Vec<String> = Vec::new();
        loop {
            let mut index = 0;    
            while startpoint + index < end_point {
                if hunted == self.merkle_tree.hashroot {
                    proof.push(hunted);
                    return Some(proof);
                }
    
                if nodes[startpoint + index] == hunted {
                    proof.push(hunted);
                    if index % 2 == 1 {
                        // it is on the right hand side
                        hunted = digest(nodes[startpoint + index - 1].clone() + &nodes[startpoint + index]) ;
                    }
                    else {
                        // it is on the left hand side
                        hunted = digest(nodes[startpoint + index].clone() + &nodes[startpoint + index + 1]);
                    }
                    startpoint = end_point;
                    end_point = end_point + steps[step_number] as usize;
                    step_number = step_number + 1;
                    index = 0;
                    continue;
                }
    
                index = index + 1;
            }
            return None;        
        }
    }

    // Use this to compare the user's proof with our's
    pub fn check_proof(&self, test :ClaimNFTParams) -> bool {
        let claimer = digest(test.node.0.iter()
            .map(|byte| format!("{:02X}", byte))
            .collect::<Vec<String>>()
            .concat());

        let master_proof = self.get_hash_proof(claimer).unwrap();
        let test_proof = test.proof;
        return master_proof == test_proof;
    }

    // Checks to see whether a given value is in the tree
    // Generally used in testing
    pub fn check_hash_value(&self, test_address :String) -> bool {
        let tree = &self.merkle_tree;
        let steps = &tree.steps;
        let mut end_point = tree.length as usize;
        let nodes = &tree.hash_tree;
        let mut hunted = test_address;
        let mut startpoint = 0;
        let mut step_number = 0;

        loop {
            let mut index: usize = 0;    
            while startpoint + index < end_point {
                if hunted.eq(&tree.hashroot) {
                    return true;
                }

                if nodes[startpoint + index] == hunted {
                    if index % 2 == 1 {
                        // it is on the right hand side
                        hunted = digest(nodes[startpoint + index - 1].clone() + &nodes[startpoint + index]) ;
                    }
                    else {
                        // it is on the left hand side
                        hunted = digest(nodes[startpoint + index].clone() + &nodes[startpoint + index + 1]);
                    }
                    startpoint = end_point;
                    end_point = end_point + steps[step_number] as usize;
                    step_number = step_number + 1;
                    index = 0;
                    continue;
                }

                index = index + 1;
            }
            return false;        
        }
    }
}


/// Your smart contract errors.
#[derive(Debug, PartialEq, Eq, Reject, Serial, SchemaType)]
enum Error {
    /// Failed parsing the parameter.
    #[from(ParseError)]
    ParseParamsError,
    NFTLimitReached,
    AddressNotOnWhitelist,
    AirdropNowClosed,
    MintingLogMalformed,
    MintingLogFull,
    MetaDataLogMalformed,
    MetaDataLogFull,
}

/// Init function that creates a new smart contract.
#[init(
contract = "airdrop_project", 
parameter = "InitParams")
]
fn init<S: HasStateApi>(
    ctx: &impl HasInitContext,
    _state_builder: &mut StateBuilder<S>,
) -> InitResult<State> {
    let params: InitParams = ctx.parameter_cursor().get()?;
    let mut state: State = State::empty();
    state.nft_limit= params.nft_limit;
    state.nft_time_limit = Some(Timestamp::from_timestamp_millis(params.nft_time_limit));
    state.nft_reserve = params.reserve;

    if params.whitelist.is_empty() == false {
        let mut whitelist: Vec<String> = vec!();
        for address in params.whitelist {
            whitelist.push(address.0.iter()
                .map(|byte| format!("{:02X}", byte))
                .collect::<Vec<String>>()
                .concat());
        }
        state.create_hash_tree(whitelist);
        state.whitelist = true;
    }
    else {
        state.whitelist = false;
    }
    Ok(state)
}


/// Claims an NFT
#[receive(
    contract = "airdrop_project",
    name = "claim_nft",
    parameter = "ClaimNFTParams",
    return_value = "ClaimReply",
    error = "Error",
    mutable,
    enable_logger,
)]
fn claim_nft<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State, StateApiType = S>,
    logger: &mut impl HasLogger,
) -> Result<ClaimReply, Error> {
    let state = host.state_mut();

    if state.nft_time_limit.is_some() {        
        let time_limit = state.nft_time_limit.unwrap();
        if time_limit > Timestamp::from_timestamp_millis(0) {
            let the_time  = ctx.metadata().slot_time();
            if the_time > state.nft_time_limit.unwrap() {
                return Err(Error::AirdropNowClosed);
            }
        }
    }

    let params: ClaimNFTParams = ctx.parameter_cursor().get()?;
    let current_token_id = state.next_token_id; 
    if current_token_id == state.nft_limit {
        return Err(Error::NFTLimitReached);
    }

    let claimer =  params.node.clone();
        
    // if there is a whitelist and no reserve only whitelist can by
    // if there is no whitelist everyone can buy
    // if there is a reserve and a whitelist only whitelist can by reserve

    if (state.whitelist == true && state.nft_reserve == 0) ||
        (state.whitelist == true && state.next_token_id >= (state.nft_limit - state.nft_reserve)) {
        if params.proof.is_empty() || state.check_proof(params) == false {
            return Err(Error::AddressNotOnWhitelist);
        }
    }   

    // Event for minted token.
    let log_mint_result = logger.log(&Cis2Event::Mint(MintEvent {
        token_id: ContractTokenId::from(current_token_id),
        amount: ContractTokenAmount::from(1),
        owner: concordium_std::Address::Account(claimer),
    }));

    match log_mint_result {
        Ok(_) => (),
        Err(error ) => {
            match  error {
                LogError::Full => {return Err(Error::MintingLogFull);},
                LogError::Malformed => {return Err(Error::MintingLogMalformed);},
            }
        } 
    }

    let url:String = state.base_url.clone() + &ContractTokenId::from(current_token_id).to_string();
    
    // Metadata URL for the token.
    let log_meta_result = logger.log(&Cis2Event::TokenMetadata::<_, ContractTokenAmount>(TokenMetadataEvent {
        token_id: ContractTokenId::from(current_token_id),
        metadata_url: MetadataUrl {
            url:  url.clone(),
            hash: None,
        },
    }));

    match log_meta_result {
        Ok(_) => (),
        Err(error ) => {
            match  error {
                LogError::Full => {return Err(Error::MetaDataLogFull);},
                LogError::Malformed => {return Err(Error::MetaDataLogMalformed);},
            }
        } 
    }

    state.next_token_id = state.next_token_id + 1;
    
    let return_value = ClaimReply {
        url : url
    };
    Ok(return_value)
}


/// View function that returns the content of the state.
#[receive(contract = "airdrop_project", name = "view", return_value = "ViewState")]
fn view<'b, S: HasStateApi>(
    _ctx: &impl HasReceiveContext,
    host: &'b impl HasHost<State, StateApiType = S>,
) -> ReceiveResult<ViewState> {
    // todo: determine what info is required here
    let view_state = ViewState {
        amount_of_claimed_tokens: host.state().next_token_id,
    };  
    Ok(view_state)
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
            whitelist:  vec!(),
            reserve:    0,
            base_url: String::new(),
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
            whitelist:  vec!(),
            reserve:    0,
            base_url: String::new(),
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
            proof: vec!(),
        };

        let mut host = TestHost::new(new_state, state_builder);

        let claim_parameter_bytes = to_bytes(&mint_params);
        ctx_claim.set_parameter(&claim_parameter_bytes);
        let mut logger = TestLogger::init();

        let claim_result = claim_nft(&ctx_claim, &mut host, &mut logger);
        assert_eq!(claim_result.is_ok(),true);

        let check = view(&ctx_claim, &host).unwrap();
        assert_eq!(check.amount_of_claimed_tokens, 1);
    
        let claim_result_bad: Result<ClaimReply, Error> = claim_nft(&ctx_claim, &mut host, &mut logger);
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
            whitelist:  whitelist.clone(),
            reserve:    0,
            base_url: String::new(),
        };

        let parameter_bytes = to_bytes(&params);
        ctx.set_parameter(&parameter_bytes);

        let state = init(&ctx, &mut state_builder).unwrap();

        // convert the addresses to strings
        let mut hashes: Vec<String> = vec!();
        for address in whitelist {
            hashes.push(digest(address.0.iter()
                .map(|byte| format!("{:02X}", byte))
                .collect::<Vec<String>>()
                .concat()));
        }

        let bad_address = ACCOUNT_3.0.iter()
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

        let test_merkle_proof = vec![hashes[0].clone(),a,c];       
        
        let test_address = digest(ACCOUNT_0.0.iter()
            .map(|byte| format!("{:02X}", byte))
            .collect::<Vec<String>>()
            .concat());
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
            whitelist:  whitelist.clone(),
            reserve:    0,
            base_url: String::new(),
        };

        let parameter_bytes = to_bytes(&params);
        ctx.set_parameter(&parameter_bytes);

        let state = init(&ctx, &mut state_builder).unwrap();

        // convert the addresses to strings
        let mut hashes: Vec<String> = vec!();
        for address in whitelist {
            hashes.push(digest(address.0.iter()
                .map(|byte| format!("{:02X}", byte))
                .collect::<Vec<String>>()
                .concat()));
        }

        let bad_address = ACCOUNT_3.0.iter()
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

        let test_merkle_proof = vec![hashes[0].clone(),a,c];       
        
        let test_address = digest(ACCOUNT_0.0.iter()
            .map(|byte| format!("{:02X}", byte))
            .collect::<Vec<String>>()
            .concat());
        let merkle_proof = state.get_hash_proof(test_address).unwrap();
        assert_eq!(merkle_proof, test_merkle_proof);

        let proof_params = ClaimNFTParams {
            proof:test_merkle_proof.clone(),
            node: ACCOUNT_0,
        };
        assert_eq!(state.check_proof(proof_params), true);

        let proof_params = ClaimNFTParams {
            proof:test_merkle_proof.clone(),
            node: ACCOUNT_1,
        };
        assert_eq!(state.check_proof(proof_params), false);
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
            whitelist:  whitelist.clone(),
            reserve:    4,            
            base_url: String::new(),
        };

        let parameter_bytes = to_bytes(&params);
        ctx.set_parameter(&parameter_bytes);

        let state = init(&ctx, &mut state_builder).unwrap();

        let mut test_proof: Vec<String> = vec!();
        let acc1 = digest(ACCOUNT_0.0.iter()
            .map(|byte| format!("{:02X}", byte))
            .collect::<Vec<String>>()
            .concat());
        let acc2 =digest(ACCOUNT_1.0.iter()
            .map(|byte| format!("{:02X}", byte))
            .collect::<Vec<String>>()
            .concat()); 
                
        test_proof.push(acc1.clone());
        test_proof.push(digest(acc1.clone() + &acc2));        

        let mut ctx_claim = TestReceiveContext::empty();
        ctx_claim.set_metadata_slot_time(Timestamp::from_timestamp_millis(1));

        let mint_params = ClaimNFTParams {
            node: ACCOUNT_0,
            proof: test_proof.clone()
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
        };
        let bad_claim_parameter_bytes = to_bytes(&mint_bad_params);
        ctx_bad_claim.set_parameter(&bad_claim_parameter_bytes);

        let claim_result_bad = claim_nft(&ctx_bad_claim, &mut host, &mut logger);
        claim_eq!(claim_result_bad, Err(Error::AddressNotOnWhitelist), "Function should fail with NFT error");
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
            whitelist:  whitelist.clone(),
            reserve:    0,            
            base_url: String::new(),
        };

        let mut test_proof: Vec<String> = vec!();
        let acc1 = digest(ACCOUNT_0.0.iter()
            .map(|byte| format!("{:02X}", byte))
            .collect::<Vec<String>>()
            .concat());
        let acc2 =digest(ACCOUNT_1.0.iter()
            .map(|byte| format!("{:02X}", byte))
            .collect::<Vec<String>>()
            .concat()); 
                
        test_proof.push(acc1.clone());
        test_proof.push(digest(acc1.clone() + &acc2));        
        
        let parameter_bytes = to_bytes(&params);
        ctx.set_parameter(&parameter_bytes);

        let state = init(&ctx, &mut state_builder).unwrap();

        let mut ctx_claim = TestReceiveContext::empty();
        let mint_params = ClaimNFTParams {
            node: ACCOUNT_0,
            proof: test_proof.clone(),
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
        };
        let bad_claim_parameter_bytes = to_bytes(&mint_bad_params);
        ctx_bad_claim.set_parameter(&bad_claim_parameter_bytes);

        let claim_result_bad = claim_nft(&ctx_bad_claim, &mut host, &mut logger);
        claim_eq!(claim_result_bad, Err(Error::AddressNotOnWhitelist), "Function should fail with NFT error");
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
            whitelist:  whitelist.clone(),
            reserve:    2,            
            base_url: String::new(),
        };

        let parameter_bytes = to_bytes(&params);
        ctx.set_parameter(&parameter_bytes);

        let state = init(&ctx, &mut state_builder).unwrap();

        let mut ctx_claim = TestReceiveContext::empty();
        ctx_claim.set_metadata_slot_time(Timestamp::from_timestamp_millis(1));

        let mint_params = ClaimNFTParams {
            node: ACCOUNT_1,
            proof: vec!(),
        };
        
        let mut host = TestHost::new(state, state_builder);

        let claim_parameter_bytes = to_bytes(&mint_params);
        ctx_claim.set_parameter(&claim_parameter_bytes);
        
        let mut logger = TestLogger::init();
        // this should not check the whitelist
        claim_nft(&ctx_claim, &mut host, &mut logger).unwrap();

        let mut ctx_wl_claim = TestReceiveContext::empty();
        ctx_wl_claim.set_metadata_slot_time(Timestamp::from_timestamp_millis(1));
        let address_hashed = digest(ACCOUNT_0.0.iter()
            .map(|byte| format!("{:02X}", byte))
            .collect::<Vec<String>>()
            .concat());
        
        let mut test_proof = vec!();
        
        test_proof.push(address_hashed.clone());
        test_proof.push(digest(address_hashed.clone()+ &address_hashed));

        let mint_wl_params = ClaimNFTParams {
            node: ACCOUNT_0,
            proof: test_proof.clone(),
        };
       
        let wl_claim_parameter_bytes = to_bytes(&mint_wl_params);
        ctx_wl_claim.set_parameter(&wl_claim_parameter_bytes);

        let mut logger = TestLogger::init();
        // this should check the whitelist and pass
        claim_nft(&ctx_wl_claim, &mut host, &mut logger).unwrap();
        
        // this should not check the whitelist and fail
        let fail_claim = claim_nft(&ctx_claim, &mut host, &mut logger);
      
        claim_eq!(fail_claim, Err(Error::AddressNotOnWhitelist), "Function should fail with whitelist error");
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
            whitelist:  vec!(),
            reserve:    0,            
            base_url: String::new(),
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
            proof: vec!(),
        };

        let mut host = TestHost::new(new_state, state_builder);

        let claim_parameter_bytes = to_bytes(&mint_params);
        ctx_claim.set_metadata_slot_time(Timestamp::from_timestamp_millis(11));
        ctx_claim.set_parameter(&claim_parameter_bytes);
        
        let mut logger = TestLogger::init();
        let claim_result = claim_nft(&ctx_claim, &mut host, &mut logger);
        claim_eq!(claim_result, Err(Error::AirdropNowClosed), "Function should fail with Airdrop closed error");
    }
}
