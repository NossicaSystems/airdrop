*Overview:*

Contract name: airdrop_project
Module reference: 1ff33abc80eaaa0467ad20107cc0daa7c31d9cb58e365e8756594c197548b074

The base64 conversion of the schema is:
//8DAQAAAA8AAABhaXJkcm9wX3Byb2plY3QBABQABgAAAAkAAAB3aGl0ZWxpc3QQAhYCCQAAAG5mdF9saW1pdAQOAAAAbmZ0X3RpbWVfbGltaXQFBwAAAHJlc2VydmUECAAAAGJhc2VfdXJsFgIOAAAAc2VsZWN0ZWRfaW5kZXgBBQAAAAsAAABjaGVja19vd25lcgIUAAEAAAAFAAAAdG9rZW4dABQAAQAAAAcAAABhZGRyZXNzFQIAAAAEAAAATm9uZQIEAAAAU29tZQEBAAAAFgIJAAAAY2xhaW1fbmZ0BBQAAwAAAAUAAABwcm9vZhACFgIEAAAAbm9kZRYCDgAAAHNlbGVjdGVkX3Rva2VuHQAVCAAAAA8AAABORlRMaW1pdFJlYWNoZWQCFQAAAEFkZHJlc3NOb3RPbldoaXRlbGlzdAIQAAAAQWlyZHJvcE5vd0Nsb3NlZAITAAAATWludGluZ0xvZ01hbGZvcm1lZAIOAAAATWludGluZ0xvZ0Z1bGwCFAAAAE1ldGFEYXRhTG9nTWFsZm9ybWVkAg8AAABNZXRhRGF0YUxvZ0Z1bGwCEwAAAEluZGV4QWxyZWFkeUNsYWltZWQCDgAAAGN1cnJlbnRfc3VwcGx5AQQMAAAAdG90YWxfc3VwcGx5AQQEAAAAdmlldwEUAAEAAAAOAAAAY2xhaW1lZF90b2tlbnMSAh0AFgIA

Please see https://www.youtube.com/watch?v=J-SP_ptKu_I&t=1999s for an example on how to use these contracts.

*External contract functions:*

Init:  This initialises the nft.    

This takes an InitParams structure which contains:
    whitelist - a vector of address.  Leave empty if there is no whitelist required.
    nft_limit - the maximum amount of nfts that can be claimed.  Leave 0 for no limit.
    nft_time_limit - the time at which the airdrop will end.
    reserve - the amount of nfts which will be held back for a whitelist (if there is no whitelist they cannot be claimed).  Leave 0 for no reserve.
    base_url - the base url for the nft


contract_claim_nft:  this claims an instance of the token.

This takes a MintParams structure which contains:
    proof - the merkle proof for the claiming node.  Can be blank if no whitelist is in use for this claim.
    node - the address of the claiming node

view:   Returns the internal state of the contract

*Intended use:*

The owner will call Init and pass through the metadata to instantiate the contract and create a claimable NFT.

Users will check in the front end if various NFTs exist and if they do they can use the contract_claim_nft to claim them.  Depending on the metadata they might need to be on the relevant whitelist.

The view function can be called to observe the current status of the nft.