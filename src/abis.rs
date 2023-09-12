use ethers::contract::abigen;

abigen!(
    WorldIDIdentityManagerImplV2,
    r#"[
        function initializeV2(address _deleteLookupTable) public
    ]"#
);

abigen!(
    WorldIDIdentityManagerImplV1,
    r#"[
        function initialize(uint8 _treeDepth, uint256 initialRoot, address _batchInsertionVerifiers, address _batchUpdateVerifiers, address _semaphoreVerifier) public
    ]"#
);

abigen!(
    VerifierLookupTable,
    r#"[
        function updateVerifier(uint256 batchSize, address verifier) public
        function disableVerifier(uint256 batchSize) public
    ]"#
);

abigen!(
    WorldIDRouterImplV1,
    r#"[
        function updateGroup(uint256 groupId, address newTargetAddress) public
        function addGroup(address groupIdentityManager) public
        function disableGroup(uint256 groupId) public
    ]"#
);
