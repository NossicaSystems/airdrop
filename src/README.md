*Overview:*

Contract name: airdrop_project
Module reference: dc178647f4e221853933eec0b875d3bf510c466bc3ea05249443ccd241dc058d

The base64 conversion of the schema is:
//8DAQAAAA8AAABhaXJkcm9wX3Byb2plY3QBABQABQAAAAkAAAB3aGl0ZWxpc3QQAgsJAAAAbmZ0X2xpbWl0BA4AAABuZnRfdGltZV9saW1pdAUHAAAAcmVzZXJ2ZQQIAAAAYmFzZV91cmwWAgIAAAAJAAAAY2xhaW1fbmZ0BhQAAgAAAAUAAABwcm9vZhACFgIEAAAAbm9kZQsUAAEAAAAIAAAAdG9rZW5faWQdABUIAAAAEAAAAFBhcnNlUGFyYW1zRXJyb3ICDwAAAE5GVExpbWl0UmVhY2hlZAIVAAAAQWRkcmVzc05vdE9uV2hpdGVsaXN0AhAAAABBaXJkcm9wTm93Q2xvc2VkAhMAAABNaW50aW5nTG9nTWFsZm9ybWVkAg4AAABNaW50aW5nTG9nRnVsbAIUAAAATWV0YURhdGFMb2dNYWxmb3JtZWQCDwAAAE1ldGFEYXRhTG9nRnVsbAIEAAAAdmlldwEEAA

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