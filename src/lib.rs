//! # A Concordium V1 smart contract
use concordium_std::*;
use concordium_cis2::*;
use sha256::digest;

use core::fmt::Debug;

/// Contract token ID type.
/// To save bytes we use a token ID type limited to a `u32`.
type ContractTokenId = TokenIdU32;

/// The parameter for the contract function `mint` which mints a number of
/// tokens to a given address.
#[derive(Serial, Deserial, SchemaType)]
struct InitParams {
    whitelist:       Vec<AccountAddress>,
    nft_limit:       u32,
    nft_limit_per_address: u32,
    nft_time_limit:  u32,
    reserve:         u32,
    token_id:        ContractTokenId,
}

/// The parameter type for the contract function `contract_claim_nft`.
#[derive(Debug, Serialize, SchemaType)]
struct ClaimNFTParams {
    owner:  AccountAddress,
    token:  ContractTokenId,
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
#[derive(Serial, DeserialWithState, StateClone)]
#[concordium(state_parameter = "S")]
pub struct State<S> {
    // Keep track of how many tokens each address holds
    all_owned_tokens:       StateMap<AccountAddress, u32, S>,
    /// All of the minted token IDs
    token_id:               ContractTokenId,
    /// Next token ID
    next_token_id:          u32,
    // Max number of nfts that can be minted before hitting reserve
    nft_limit_per_address:  u32,
    // Max number of nfts that can be minted before hitting reserve
    nft_limit:              u32,
    // Number of nfts which are held in reserve
    nft_reserve:            u32,    
    // Airdrop time limit in blocks
    nft_time_limit:         u32,
    // Whitelist proof
    whitelist:              bool,
    merkle_tree:            MerkleTree,
}

impl<S: HasStateApi> State<S> {
    /// Creates a new state with no tokens.
    fn empty(state_builder: &mut StateBuilder<S>) -> Self {
        State {
            all_owned_tokens: state_builder.new_map(), 
            token_id:    TokenIdU32(0),
            next_token_id: 0,
            nft_limit:     1,
            nft_limit_per_address: 0,
            merkle_tree:    MerkleTree::new(),
            whitelist:      false,
            nft_time_limit: 0,
            nft_reserve: 0,
        }
    }

    /// Mint a new token with a given address as the owner
    fn mint(
        &mut self,
        claimer: &AccountAddress,
    ) -> bool {
                
        // Insert or replace the vote for the account.
        self
            .all_owned_tokens
            .entry(*claimer)
            .and_modify(|prev_valule| *prev_valule = *prev_valule + 1)
            .or_insert(1);
    
        true
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
       //                 proof.push(nodes[startpoint + index - 1].clone());
                    }
                    else {
                        // it is on the left hand side
                        hunted = digest(nodes[startpoint + index].clone() + &nodes[startpoint + index + 1]);
         //               proof.push(nodes[startpoint + index + 1].clone());
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

    // Checks to see whether a given value is in the tree
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
}

/// Init function that creates a new smart contract.
#[init(
contract = "airdrop_project", 
parameter = "InitParams")
]
fn init<S: HasStateApi>(
    ctx: &impl HasInitContext,
    state_builder: &mut StateBuilder<S>,
) -> InitResult<State<S>> {
    let params: InitParams = ctx.parameter_cursor().get()?;
    let mut state: State<S> = State::empty(state_builder);
    state.nft_limit= params.nft_limit;
    state.nft_time_limit = params.nft_time_limit;
    state.token_id = params.token_id;
    state.nft_reserve = params.reserve;
    state.nft_limit_per_address = params.nft_limit_per_address;

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
/// It rejects if:
#[receive(
    contract = "airdrop_project",
    name = "contract_claim_nft",
    parameter = "ClaimNFTParams",
    error = "Error",
    mutable
)]
fn contract_claim_nft<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> Result<bool, Error> {
    let state = host.state_mut();
    let params: ClaimNFTParams = ctx.parameter_cursor().get()?;
    
    if state.next_token_id == state.nft_limit {
        return Err(Error::NFTLimitReached);
    }

    let address_string =  params.owner.0.iter()
        .map(|byte| format!("{:02X}", byte))
        .collect::<Vec<String>>()
        .concat();
        
    let test_hash = digest(address_string);
    // if there is a whitelist and no reserve only whitelist can by
    // if there is no whitelist everyone can buy
    // if there is a reserve and a whitelist only whitelist can by reserve

    if (state.whitelist == true && state.nft_reserve == 0) ||
        (state.whitelist == true && state.next_token_id > (state.nft_limit - state.nft_reserve)) {
        if state.check_hash_value(test_hash) == false {
            return Err(Error::AddressNotOnWhitelist);
        }
    }   
  
    let max_claims = state.nft_limit_per_address;
    if max_claims != 0 {
        match state.all_owned_tokens.get(&params.owner) {
            Some(val) => {
                
                if *val >= max_claims {
                    return Err(Error::NFTLimitReached);
                }
            },
            None => {},
        };
    }
    let res = state.mint(&params.owner);

    if res == true {
        state.next_token_id = state.next_token_id + 1;
    }

    Ok(res)
}


/* 
/// Receive function. The input parameter is the boolean variable `throw_error`.
///  If `throw_error == true`, the receive function will throw a custom error.
///  If `throw_error == false`, the receive function executes successfully.
#[receive(
    contract = "airdrop_project",
    name = "receive",
    parameter = "bool",
    error = "Error",
    mutable
)]
fn receive<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    _host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> Result<(), Error> {
    // Your code

    let throw_error = ctx.parameter_cursor().get()?; // Returns Error::ParseError on failure
    if throw_error {
        Err(Error::YourError)
    } else {
        Ok(())
    }
}*/

/// View function that returns the content of the state.
#[receive(contract = "airdrop_project", name = "view", return_value = "State")]
fn view<'b, S: HasStateApi>(
    _ctx: &impl HasReceiveContext,
    host: &'b impl HasHost<State<S>, StateApiType = S>,
) -> ReceiveResult<&'b State<S>> {
    Ok(host.state())
}

#[concordium_cfg_test]
mod tests {
    use super::*;
    use test_infrastructure::*;

    #[concordium_test] 
    /// Test that initializing the contract succeeds with some state.
    fn test_init() {
        let mut ctx = TestInitContext::empty();

        let mut state_builder = TestStateBuilder::new();
        
        let params = InitParams {
            nft_limit: 0,
            nft_time_limit: 0,
            nft_limit_per_address: 0,
            whitelist:  vec!(),
            reserve:    0,
            token_id:  TokenIdU32(0),
        };

        let parameter_bytes = to_bytes(&params);
        ctx.set_parameter(&parameter_bytes);

        let state_result = init(&ctx, &mut state_builder);
        let res = state_result.unwrap();        
        assert_eq!(res.all_owned_tokens.is_empty(), true);
        assert_eq!(res.token_id, TokenIdU32(0));
        assert_eq!(res.nft_limit, 0);
        assert_eq!(res.nft_time_limit, 0);
    }

    #[concordium_test]
    fn test_mint_no_reserve_no_whitelist() {
        let mut ctx = TestInitContext::empty();

        let mut state_builder = TestStateBuilder::new();
        
        const ACCOUNT_0: AccountAddress = AccountAddress([0u8; 32]);
        const TOKEN_1: ContractTokenId = TokenIdU32(1);
    
        // This should allow anyone to purchase 1 NFT of TOKEN_0
        let params = InitParams {
            nft_limit: 1,
            nft_time_limit: 0,
            nft_limit_per_address: 0,
            whitelist:  vec!(),
            reserve:    0,
            token_id: TOKEN_1,
        };

        let parameter_bytes = to_bytes(&params);
        ctx.set_parameter(&parameter_bytes);

        let state_result = init(&ctx, &mut state_builder);
        let new_state = state_result.unwrap();        
        assert_eq!(new_state.all_owned_tokens.is_empty(), true);
        assert_eq!(new_state.token_id, TOKEN_1);
        assert_eq!(new_state.nft_limit, 1);
        assert_eq!(new_state.nft_time_limit, 0);

        let mut ctx_claim = TestReceiveContext::empty();
        let mint_params = ClaimNFTParams {
            token: TOKEN_1,
            owner: ACCOUNT_0,
        };

        let mut host = TestHost::new(new_state, state_builder);

        let claim_parameter_bytes = to_bytes(&mint_params);
        ctx_claim.set_parameter(&claim_parameter_bytes);
        
        let claim_result = contract_claim_nft(&ctx_claim, &mut host).unwrap();
        assert_eq!(claim_result,true);

        let check = view(&ctx_claim, &host).unwrap();
        match check.all_owned_tokens.get(&ACCOUNT_0) {
            Some(val) => {assert_eq!(*val, 1);},
            None => {assert!(false, "Address did not mint contract")},
        }
        
        assert_eq!(check.next_token_id, 1);
    
        let claim_result_bad = contract_claim_nft(&ctx_claim, &mut host);
        claim_eq!(claim_result_bad, Err(Error::NFTLimitReached), "Function should fail with NFT error");
    }

    #[concordium_test]
    fn test_whitelist() {
        let mut ctx = TestInitContext::empty();
        let mut state_builder = TestStateBuilder::new();
        
        const ACCOUNT_0: AccountAddress = AccountAddress([0u8; 32]);
        const ACCOUNT_1: AccountAddress = AccountAddress([1u8; 32]);
        const ACCOUNT_2: AccountAddress = AccountAddress([2u8; 32]);
        const ACCOUNT_3: AccountAddress = AccountAddress([3u8; 32]);
        const TOKEN_1: ContractTokenId = TokenIdU32(1);

        let whitelist: Vec<AccountAddress> = vec![ACCOUNT_0, ACCOUNT_1, ACCOUNT_2];
        
        // This should allow anyone to purchase 1 NFT of TOKEN_0
        let params = InitParams {
            nft_limit: 1,
            nft_time_limit: 0,
            nft_limit_per_address: 0,
            whitelist:  whitelist.clone(),
            reserve:    0,
            token_id: TOKEN_1,
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
    fn test_claim_with_whitelist_full_reserve() {
        let mut ctx = TestInitContext::empty();
        let mut state_builder = TestStateBuilder::new();
        
        const ACCOUNT_0: AccountAddress = AccountAddress([0u8; 32]);
        const ACCOUNT_1: AccountAddress = AccountAddress([1u8; 32]);
        const ACCOUNT_2: AccountAddress = AccountAddress([2u8; 32]);
        const TOKEN_1: ContractTokenId = TokenIdU32(1);

        let whitelist: Vec<AccountAddress> = vec![ACCOUNT_0, ACCOUNT_1];
        
        // This should allow anyone to purchase 1 NFT of TOKEN_0
        let params = InitParams {
            nft_limit: 4,
            nft_time_limit: 0,
            nft_limit_per_address: 0,
            whitelist:  whitelist.clone(),
            reserve:    4,
            token_id: TOKEN_1,
        };

        let parameter_bytes = to_bytes(&params);
        ctx.set_parameter(&parameter_bytes);

        let state = init(&ctx, &mut state_builder).unwrap();

        let mut ctx_claim = TestReceiveContext::empty();
        let mint_params = ClaimNFTParams {
            token: TOKEN_1,
            owner: ACCOUNT_0,
        };
        
        let mut host = TestHost::new(state, state_builder);

        let claim_parameter_bytes = to_bytes(&mint_params);
        ctx_claim.set_parameter(&claim_parameter_bytes);
        
        contract_claim_nft(&ctx_claim, &mut host).unwrap();

        let mut ctx_bad_claim = TestReceiveContext::empty();
        let mint_bad_params = ClaimNFTParams {
            token: TOKEN_1,
            owner: ACCOUNT_2,
        };
        let bad_claim_parameter_bytes = to_bytes(&mint_bad_params);
        ctx_bad_claim.set_parameter(&bad_claim_parameter_bytes);

        let claim_result_bad = contract_claim_nft(&ctx_bad_claim, &mut host);
        claim_eq!(claim_result_bad, Err(Error::AddressNotOnWhitelist), "Function should fail with NFT error");
    }

    #[concordium_test]
    fn test_claim_with_whitelist_no_reserve() {
        let mut ctx = TestInitContext::empty();
        let mut state_builder = TestStateBuilder::new();
        
        const ACCOUNT_0: AccountAddress = AccountAddress([0u8; 32]);
        const ACCOUNT_1: AccountAddress = AccountAddress([1u8; 32]);
        const ACCOUNT_2: AccountAddress = AccountAddress([2u8; 32]);
        const TOKEN_1: ContractTokenId = TokenIdU32(1);

        let whitelist: Vec<AccountAddress> = vec![ACCOUNT_0, ACCOUNT_1];
        
        // This should allow anyone to purchase 1 NFT of TOKEN_0
        let params = InitParams {
            nft_limit: 4,
            nft_time_limit: 0,
            nft_limit_per_address: 0,
            whitelist:  whitelist.clone(),
            reserve:    0,
            token_id: TOKEN_1,
        };

        let parameter_bytes = to_bytes(&params);
        ctx.set_parameter(&parameter_bytes);

        let state = init(&ctx, &mut state_builder).unwrap();

        let mut ctx_claim = TestReceiveContext::empty();
        let mint_params = ClaimNFTParams {
            token: TOKEN_1,
            owner: ACCOUNT_0,
        };
        
        let mut host = TestHost::new(state, state_builder);

        let claim_parameter_bytes = to_bytes(&mint_params);
        ctx_claim.set_parameter(&claim_parameter_bytes);
        
        contract_claim_nft(&ctx_claim, &mut host).unwrap();

        let mut ctx_bad_claim = TestReceiveContext::empty();
        let mint_bad_params = ClaimNFTParams {
            token: TOKEN_1,
            owner: ACCOUNT_2,
        };
        let bad_claim_parameter_bytes = to_bytes(&mint_bad_params);
        ctx_bad_claim.set_parameter(&bad_claim_parameter_bytes);

        let claim_result_bad: Result<bool, Error> = contract_claim_nft(&ctx_bad_claim, &mut host);
        claim_eq!(claim_result_bad, Err(Error::AddressNotOnWhitelist), "Function should fail with NFT error");
    }


    #[concordium_test]
    fn test_claim_with_whitelist_partial_reserve() {
        let mut ctx = TestInitContext::empty();
        let mut state_builder = TestStateBuilder::new();
        
        const ACCOUNT_0: AccountAddress = AccountAddress([0u8; 32]);
        const ACCOUNT_1: AccountAddress = AccountAddress([1u8; 32]);
        const TOKEN_1: ContractTokenId = TokenIdU32(1);

        let whitelist: Vec<AccountAddress> = vec![ACCOUNT_0];
        
        // This should allow anyone to purchase 1 NFT of TOKEN_0
        let params = InitParams {
            nft_limit: 3,
            nft_time_limit: 0,
            nft_limit_per_address: 0,
            whitelist:  whitelist.clone(),
            reserve:    2,
            token_id: TOKEN_1,
        };

        let parameter_bytes = to_bytes(&params);
        ctx.set_parameter(&parameter_bytes);

        let state = init(&ctx, &mut state_builder).unwrap();

        let mut ctx_claim = TestReceiveContext::empty();
        let mint_params = ClaimNFTParams {
            token: TOKEN_1,
            owner: ACCOUNT_1,
        };
        
        let mut host = TestHost::new(state, state_builder);

        let claim_parameter_bytes = to_bytes(&mint_params);
        ctx_claim.set_parameter(&claim_parameter_bytes);
        
        contract_claim_nft(&ctx_claim, &mut host).unwrap();

        let mut ctx_wl_claim = TestReceiveContext::empty();
        let mint_wl_params = ClaimNFTParams {
            token: TOKEN_1,
            owner: ACCOUNT_0,
        };
        let wl_claim_parameter_bytes = to_bytes(&mint_wl_params);
        ctx_wl_claim.set_parameter(&wl_claim_parameter_bytes);

        let good_claim = contract_claim_nft(&ctx_wl_claim, &mut host).unwrap();
        assert_eq!(good_claim, true);
        let fail_claim = contract_claim_nft(&ctx_claim, &mut host);
      
        claim_eq!(fail_claim, Err(Error::AddressNotOnWhitelist), "Function should fail with NFT error");
    }


    #[concordium_test]
    fn test_mint_more_than_allowed_per_address() {
        let mut ctx = TestInitContext::empty();

        let mut state_builder = TestStateBuilder::new();
        
        const ACCOUNT_0: AccountAddress = AccountAddress([0u8; 32]);
        const TOKEN_1: ContractTokenId = TokenIdU32(1);
    
        // This should allow anyone to purchase 1 NFT of TOKEN_0
        let params = InitParams {
            nft_limit: 10,
            nft_time_limit: 0,
            nft_limit_per_address: 1,
            whitelist:  vec!(),
            reserve:    0,
            token_id: TOKEN_1,
        };

        let parameter_bytes = to_bytes(&params);
        ctx.set_parameter(&parameter_bytes);

        let state_result = init(&ctx, &mut state_builder);
        let new_state = state_result.unwrap();        
 
        let mut ctx_claim = TestReceiveContext::empty();
        let mint_params = ClaimNFTParams {
            token: TOKEN_1,
            owner: ACCOUNT_0,
        };

        let mut host = TestHost::new(new_state, state_builder);

        let claim_parameter_bytes = to_bytes(&mint_params);
        ctx_claim.set_parameter(&claim_parameter_bytes);
        
        let _ = contract_claim_nft(&ctx_claim, &mut host).unwrap();
        let claim_result_bad = contract_claim_nft(&ctx_claim, &mut host);
        claim_eq!(claim_result_bad, Err(Error::NFTLimitReached), "Function should fail with NFT error");
    }
}
