*Overview:*

Contract name: airdrop_project
Module reference: 4d25cfad64199252b47ebcdd5dcf1ac7e05a8cdf386d78cb7b37873592e8b412
Please see https://www.youtube.com/watch?v=J-SP_ptKu_I&t=1999s for an example on how to use these contracts.

*External contract functions:*

Init:  This initialises the nft.

This takes an InitParams structure which contains:
    whitelist - a vector of address.  Leave empty if there is no whitelist required.
    nft_limit - the maximum amount of nfts that can be claimed.  Leave 0 for no limit.
    nft_limit_per_address - the maximum amount of nfts that an individual address can claim.  Leave 0 for no limit.
    nft_time_limit - the amount of blocks for which the airdrop will last.
    reserve - the amount of nfts which will be held back for a whitelist (if there is no whitelist they cannot be claimed).  Leave 0 for no reserve.
    token_id - the id of the nft


contract_claim_nft:  this claims an instance of the token.

This takes a MintParams structure which contains:
    proof - the merkle proof for the claiming node.  Can be blank if no whitelist is in use for this claim.
    node - the hashed address of the claiming node

view:   Returns the internal state of the contract

*Intended use:*

The owner will call Init and pass through the metadata to instantiate the contract and create a claimable NFT.

Users will check in the front end if various NFTs exist and if they do they can use the contract_claim_nft to claim them.  Depending on the metadata they might need to be on the relevant whitelist.

The view function can be called to observe the current status of the nft.