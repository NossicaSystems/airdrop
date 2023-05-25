*Overview:*

Contract name: airdrop_project
Module reference: 7c582db1ef2fa0ecf1837694510df86f605c5b84dc105db8bd5c138efc4fe522

The base64 conversion of the schema is:
//8DAQAAAA8AAABhaXJkcm9wX3Byb2plY3QBABQABgAAAAkAAAB3aGl0ZWxpc3QQAgsJAAAAbmZ0X2xpbWl0BBUAAABuZnRfbGltaXRfcGVyX2FkZHJlc3MEDgAAAG5mdF90aW1lX2xpbWl0BAcAAAByZXNlcnZlBAgAAAB0b2tlbl9pZB0AAgAAABIAAABjb250cmFjdF9jbGFpbV9uZnQEFAACAAAABQAAAHByb29mEAIWAgQAAABub2RlFgIVAwAAABAAAABQYXJzZVBhcmFtc0Vycm9yAg8AAABORlRMaW1pdFJlYWNoZWQCFQAAAEFkZHJlc3NOb3RPbldoaXRlbGlzdAIEAAAAdmlldwEUAAEAAAAYAAAAYW1vdW50X29mX2NsYWltZWRfdG9rZW5zBAA

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