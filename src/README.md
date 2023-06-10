*Overview:*

Contract name: airdrop_project
Module reference: 78d89398f3e820c50932ab848c90665e080b9bfc4ca1b4cb2dcb92c66254ec7f

The base64 conversion of the schema is:
//8DAQAAAA8AAABhaXJkcm9wX3Byb2plY3QBABQACQAAAAkAAAB3aGl0ZWxpc3QQAhYCCQAAAG5mdF9saW1pdAQVAAAAbmZ0X2xpbWl0X3Blcl9hZGRyZXNzBA4AAABuZnRfdGltZV9saW1pdAUHAAAAcmVzZXJ2ZQQIAAAAYmFzZV91cmwWAggAAABtZXRhZGF0YRYCDgAAAHdoaXRlbGlzdF9maWxlFgIOAAAAc2VsZWN0ZWRfaW5kZXgBBgAAAAoAAABiYWxhbmNlX29mAhQAAgAAAAYAAABfZHVtbXkIBAAAAG5vZGULBAsAAABjaGVja19vd25lcgIUAAEAAAAFAAAAdG9rZW4dABQAAQAAAAcAAABhZGRyZXNzFQIAAAAEAAAATm9uZQIEAAAAU29tZQEBAAAAFgIJAAAAY2xhaW1fbmZ0BBQABQAAAAUAAABwcm9vZhACFgIEAAAAbm9kZQsLAAAAbm9kZV9zdHJpbmcWAg4AAABzZWxlY3RlZF90b2tlbh0AEAAAAGFtb3VudF9vZl90b2tlbnMEFQgAAAAPAAAATkZUTGltaXRSZWFjaGVkAhUAAABBZGRyZXNzTm90T25XaGl0ZWxpc3QCEAAAAEFpcmRyb3BOb3dDbG9zZWQCEwAAAE1pbnRpbmdMb2dNYWxmb3JtZWQCDgAAAE1pbnRpbmdMb2dGdWxsAhQAAABNZXRhRGF0YUxvZ01hbGZvcm1lZAIPAAAATWV0YURhdGFMb2dGdWxsAhMAAABJbmRleEFscmVhZHlDbGFpbWVkAg4AAABjdXJyZW50X3N1cHBseQEEDAAAAHRvdGFsX3N1cHBseQEEBAAAAHZpZXcBFAADAAAACAAAAG1ldGFkYXRhFgIJAAAAd2hpdGVsaXN0FgIOAAAAbnVtYmVyX29mX25mdHMEAA

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