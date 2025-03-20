# Async Interaction Patterns with ZK Recursion

This document describes a system that performs recursive zero-knowledge computation enabling verifiable off-chain computation with on-chain anchoring. The system uses a ZK coprocessor operated by a dedicated actor to perform computations, generate proofs, and eventually submit results to the blockchain. For simplicity the Prover, which may be a distinct role in practice, is collapsed into the operator.

#### Example Recusion Flow

1. At block height 0, an on-chain commitment is made with a zk-nonce value of 10.

2. A coprocessor operator observes this commitment and uses it as a public input to the ZK coprocessor.

3. The coprocessor performs a computation that increments the zk-nonce, resulting in a new value of 11. This result remains off-chain at this point.

4. At block height 1, a trader submits a transaction that updates a market price to 5. This transaction is included in the blockchain.

5. The coprocessor operator observes this price update and generates a light client proof verifying the on-chain state.

6. The operator now submits to the coprocessor:
   - The previous computation output (zk-nonce=11)
   - The market state (price=5)
   - The light client proof verifying the market state

7. The coprocessor performs a second computation, resulting in zk-nonce=12. This result again remains off-chain.

8. A user observes the current market price (5) and creates a signed transaction with the following conditions:
   - The zk-nonce must be less than 13
   - The price must be 5
   - The inclusion block height must be ≤ (user's observed block height + 3)
   - If conditions are met, execute a purchase effect

9. The user sends this signed transaction to the coprocessor operator, not directly to the chain.

10. The operator submits to the coprocessor:
    - The previous computation output (zk-nonce=12)
    - All previous public inputs (including the market price)
    - The user's signed transaction

11. The coprocessor performs a third computation, resulting in zk-nonce=13 and a message to execute the user's purchase order.

12. The operator then submits this final result to the blockchain for inclusion at block height 4.

13. The transaction is verified on-chain because:
    - The zk-nonce (13) is valid based on user's condition (< 13)
    - The block height (4) is within the acceptable range
    - The price (5) matches the user's condition
    - The light client proof validates that the price data came from a legitimate block
    - The recursive ZK proof validates the entire computation chain

## Sequence Diagram

This sequence diagram illustrates the interplay between on-chain state, coprocessor operators, the ZK coprocessor itself, users, and other actors able to change blockchain state, illustrated here with traders. Notice the clear separation between on-chain events (which require consensus) and off-chain computation (which requires only ZK verification). The critical paths show how the operator bridges these two domains, gathering inputs, submitting them to the coprocessor, and eventually bringing the results back on-chain.

```mermaid
sequenceDiagram
    participant OnChainState as On-Chain State
    participant CoprocessorOp as Coprocessor Operator
    participant Coprocessor as Coprocessor
    participant User as User
    participant Trader as Trader

    OnChainState->>OnChainState: 1. Initialize commitment (zk-nonce=10) at block 0
    CoprocessorOp->>OnChainState: 2. Observe commitment from block 0
    CoprocessorOp->>Coprocessor: 3. Submit (zk-nonce=10) as public input
    Coprocessor->>Coprocessor: 4. Compute (zk-nonce=11)
    Coprocessor->>CoprocessorOp: 5. Receive computation result
    CoprocessorOp->>CoprocessorOp: 6. Store (zk-nonce=11) locally
    
    OnChainState->>OnChainState: 7. Market price update (price=5) at block 1
    CoprocessorOp->>OnChainState: 8. Observe price update
    CoprocessorOp->>CoprocessorOp: 9. Generate light client proof for price
    CoprocessorOp->>Coprocessor: 10. Submit (zk-nonce=11), (price=5), light client proof as input
    Coprocessor->>Coprocessor: 11. Compute (zk-nonce=12)
    Coprocessor->>CoprocessorOp: 12. Receive computation result
    CoprocessorOp->>CoprocessorOp: 13. Store (zk-nonce=12) locally

    Trader->>Trader: 14. Observe market price
    Trader->>Trader: 15. Sign conditional order
    Trader->>CoprocessorOp: 16. Receive user order message
    CoprocessorOp->>Coprocessor: 17. Submit (zk-nonce=12), previous inputs, user order as inputs
    Coprocessor->>Coprocessor: 18. Compute (zk-nonce=13) + order execution
    Coprocessor->>CoprocessorOp: 19. Receive computation result with order info
    CoprocessorOp->>OnChainState: 20. Submit transaction with order and proof
    OnChainState->>OnChainState: 21. Execute order + update zk-nonce (zk-nonce=13) at block 4
```

The diagram highlights the asynchronous nature of the system—the user signs their order based on observed market conditions without direct knowledge of the computational state. Meanwhile, the operator must ensure all conditions remain valid when finally submitting to the chain.

| Step | Public Inputs | Private Witness Data | Verification Key Used | Proof Generated | Notes |
|------|---------------|---------------------|----------------------|-----------------|-------|
| **Initial Commitment** (Block 0) | • Initial nonce value (10) | • None (on-chain value) | • None (direct on-chain state) | • None | This is the starting point anchored on-chain |
| **First Computation** | • Initial nonce (10) | • Internal coprocessor state<br>• Computation steps for incrementing nonce | • Primary circuit VK<br>(`verify_nonce_update`) | • Proof that nonce 10 was correctly incremented to 11 | This proof remains off-chain until later submission |
| **Price Update** (Block 1) | • Market price (5) | • None (on-chain value) | • None (direct on-chain state) | • None | External state change on blockchain |
| **Light Client Proof Generation** | • Block header from Block 1<br>• Merkle branch to price data | • Full block data<br>• Merkle tree structure | • Light client VK<br>(`verify_eth_state_proof`) | • Proof that price=5 exists in Block 1 state | This connects on-chain state to off-chain computation |
| **Second Computation** | • Previous computation result (nonce=11)<br>• Market price (5)<br>• Block 1 header data | • Previous computation witness<br>• Light client proof witness<br>• Computation steps for price processing | • Recursive circuit VK<br>(`verify_computation_and_state`) | • Proof that given nonce=11 and price=5, the computation correctly produced nonce=12 | First recursive step, combining previous result with new data |
| **User Order Creation** | • User-observed price (5)<br>• User-observed block height | • User's private key | • Signature verification<br>(ECDSA or similar) | • Digital signature on order | Created by user, not part of ZK computation yet |
| **Third Computation** | • Previous computation result (nonce=12)<br>• All previous public inputs<br>• User order with conditions<br>• User signature<br>• Current block height reference | • Previous computation witness<br>• User order verification witness<br>• Computation steps for order processing | • Recursive circuit VK<br>(`verify_computation_order`) | • Proof that given nonce=12, price=5, and valid user order, the computation correctly produced nonce=13 and order execution message | Most complex step, validating multiple conditions and recursively building on previous proof |
| **On-chain Submission** (Block 4) | • Final computation result (nonce=13)<br>• User order execution message<br>• Block references for light client proofs | • Complete recursive ZK proof | • Final verification key<br>(registered on-chain) | • N/A (using the proof, not generating one) | The final proof and public inputs are submitted on-chain for verification |

## Actor Discretion

This table maps the boundaries of each actor's discretion within the system. Coprocessor operators have significant discretion over timing and input selection but cannot manipulate the actual computation. Users have early discretion when formulating orders but surrender control once submitted. Traders influence the system only through price-setting actions, while the coprocessor itself has zero discretion, serving as the deterministic core of the system. The blockchain's discretion is limited to transaction ordering within consensus rules.

| Actor | Discretion | Constraints | Evolution over time |
|-------|-------------|-------------|---------------------|
| **Coprocessor operator** | • Timing of observation<br>• Selection of inputs to include<br>• Order of computation<br>• When to submit results | • Cannot modify input content<br>• Cannot alter coprocessor logic<br>• Must include valid light client proofs | Discretion narrows as user conditions expire or become invalid |
| **User** | • Order parameters<br>• Conditional constraints<br>• When to submit orders<br>• Can issue superseding orders | • Cannot control execution timing<br>• Cannot retroactively invalidate signed orders | Discretion ends after order submission, remaining influence limited to potential superseding orders |
| **Trader** | • Price setting<br>• Timing of trades | • Subject to market mechanisms<br>• Cannot directly control off-chain computation | Discretion is punctuated - high at moment of trade, zero otherwise |
| **Coprocessor (ZK circuit)** | • None - deterministic computation | • Must follow predefined logic<br>• Inputs fully determine outputs | No discretion at any point |
| **Blockchain** | • Block production timing<br>• Transaction inclusion | • Must follow consensus rules<br>• Cannot reject valid transactions | Discretion is constant but constrained by protocol rules |

The evolution of discretion over time is worth noting - user influence frontloads into order creation and gradually diminishes, while operator discretion narrows as conditions approach expiration. This temporal shift in control creates unique game-theoretic dynamics.

### 2. Causal Understanding and Information Asymmetry

In this table we can see the degree of information asymmetry between actors in the system. Operators maintain the most complete picture, with visibility into both on-chain state and off-chain computation. Users operate with limited information about computation state, creating an inherent uncertainty about when and if their orders will execute. Traders influence the system but have minimal visibility into how their actions affect computation. The blockchain itself has the most limited view, knowing only what is committed on-chain.


| Actor | Knowledge of System State | Information Asymmetries | Evolution of Understanding |
|-------|---------------------------|-------------------------|----------------------------|
| **Coprocessor operator** | • Complete view of on-chain state<br>• Full knowledge of previous computations<br>• Awareness of pending user orders | • Cannot know about competing operators' computations until they're submitted<br>• Limited visibility into users' future actions | • Builds a continuously updating causal graph<br>• Information becomes more complete as block height increases |
| **User** | • Access to on-chain state<br>• Knowledge of their own orders<br>• No direct visibility into computation state | • No visibility into operator's computation queue<br>• Cannot know if their order will be included in current recursion<br>• Limited knowledge of other users' actions | • Point-in-time snapshot when creating order<br>• Infers state changes after transaction confirmation |
| **Trader** | • Knowledge of their own trades<br>• On-chain market state | • No visibility into off-chain computation<br>• Limited knowledge of pending orders | • Primarily concerned with immediate market impact<br>• May monitor effects post-trade |
| **Blockchain** | • Canonical record of committed state<br>• No "understanding" of off-chain logic | • No visibility into computation process<br>• Only verifies submitted proofs | • Linear accumulation of state<br>• No causal inference capabilities |

These asymmetries shape strategy and incentives within the system. Operators might batch computations to amortize costs, users might create more flexible conditions to increase execution probability, and traders might monitor post-trade effects to improve future strategy. The evolution of each actor's understanding varies drastically, from the operator's continuously updating view to the user's discrete state observations.

### 3. Provability Matrix

This provability matrix shows how knowledge and verification capabilities evolve across the computation lifecycle. 

| Stage | Coprocessor operator | User | Trader | On-chain verifier |
|-------|----------------------|------|--------------|-------------------|
| **After initial commitment** | • zk-nonce value | • Current on-chain state | • Current price | • Committed zk-nonce |
| **After first computation** | • zk-nonce increment is valid<br>• Computation was correct | • Nothing about off-chain computation | • Nothing about off-chain computation | • Nothing yet |
| **After price update** | • Original zk-nonce valid<br>• Price update occurred<br>• Light client proof is valid | • Current on-chain price | • Trade was executed | • Updated price |
| **After second computation** | • zk-nonce incremented correctly<br>• Price was incorporated<br>• Full computational integrity | • Nothing about off-chain computation | • Nothing about off-chain computation | • Nothing yet |
| **After user order** | • User authorized order<br>• Order meets user conditions<br>• Order can be validly executed | • Order was properly signed<br>• Conditions are valid | • Nothing about pending orders | • Nothing yet |
| **After final computation** | • Complete execution trace is valid<br>• All conditions satisfied<br>• Final zk-nonce is correct | • Nothing until on-chain confirmation | • Nothing about pending orders | • Nothing yet |
| **After on-chain submission** | • Transaction is finalized | • Order was executed<br>• Execution respected conditions | • Order impact on market | • Complete proof validity<br>• All conditions satisfied<br>• zk-nonce updated correctly |

The operator's proof capabilities throughout the process contrast with the limited verification abilities of other actors until final on-chain submission. 

Users, for instance, can only prove facts about their own order submission until the final on-chain commitment provides external verification. Similarly, traders have limited ability to prove anything about the computation despite their price-setting actions being critical inputs.

This asymmetry in provability creates interesting trust dynamics. The system is designed so that complete verification is available once results hit the chain, but prior to that point, different actors have drastically different abilities to verify system state and behavior.

## Cross-chain Message Extension

The system's handling of user-signed messages can be naturally extended to incorporate authenticated messages from other blockchains. Cross-chain messages function as "blockchain-signed messages" in contrast to user-signed messages. They follow the same pattern of authenticated, conditional instructions that are incorporated into the recursive computation, but with different authentication mechanisms and timing properties. From the coprocessor's perspective, both are simply authenticated instructions with conditions attached.

## Causal Evolution

The following diagram illustrates the causal dependencies between events in the system:

```mermaid
graph TD;
    subgraph B1["Block 0"]
        B1A["zk-nonce = 10"]
    end

    subgraph B2["Block 1"]
        B2A["Market price = 5"]
    end

    subgraph B3["Block 2"]
        B3A[" "]
    end

    subgraph B4["Block 3"]
        
        B4B["Public inputs<br/>Proof"]
        B4D["Execute order"]
    end

    subgraph Operator["Operator"]
        ZK11["zk-nonce = 11<br/>Light client proof"]
        B4A["zk-nonce = 12"]
        B4C["zk-nonce = 13<br/>Order inclusion"]
    end

    subgraph User["User"]
        G["Create purchase order:<br>if zk-nonce < 13<br>if price = 5<br>if block ≤ current+3"]
    end

    B1A -->|Computation 1| ZK11
    B2A -->|Observes:<br/>price = 5<br/>block header| ZK11

    ZK11 -->|Computation 2| B4A

    B2A -->|Observes:<br/>price = 5| G
    G -->|Submission| B4C
    B4A -->|Computation 3| B4C
    B4C -->|Submission| B4B
    B4B --> B4D

    B1 --> B2 --> B3 --> B4
```

## Design Considerations

1. **Operator Competition**: Multiple operators may compete to have their computation accepted on-chain, potentially leading to wasted work. Design considerations:
   - Gas price strategies for prioritization
   - Economic incentives for operator specialization
   - Potential for operator collaboration mechanisms

2. **Proof Aggregation Efficiency**: As the recursion depth increases, proof generation becomes more complex. Important factors:
   - Optimization of proof circuits
   - Batching strategies for related computations
   - Tradeoffs between recursion depth and proof size

3. **Temporal Constraints**: Time-bound conditions create deadline pressures. Considerations:
   - Balancing window sizes for operator flexibility vs user certainty
   - Handling chain reorganizations that might invalidate conditions
   - Strategies for conditions that span multiple block ranges

4. **Failure Modes**:
   - Light client proof invalidation due to deep reorganizations
   - Competing operators causing computation invalidation
   - Malicious operators withholding computations to force condition timeouts

## Applications and Extensions

This recursive ZK computation model enables several applications:

1. **Cross-Chain DeFi**: Atomic operations spanning multiple blockchains with verifiable execution.

2. **Conditional Privacy (assuming client-side proving)**: Transactions that remain private until specific conditions are met.

3. **Batched Optimistic Settlement**: Multiple operations batched together with optimistic execution and ZK fallback verification.

4. **Time-Bound Commitments**: Commitments that automatically execute or expire based on temporal conditions.

5. **Verifiable Event Reaction**: Programmatic, verifiable reactions to on-chain events without requiring on-chain computation for the reaction logic.

## Conclusion

The recursive ZK computation system creates a powerful bridge between off-chain computation and on-chain verification. By leveraging zero-knowledge proofs, light clients, and conditional execution, it enables complex computational flows that maintain verifiability without requiring all computation to occur on-chain.

By balancing actor discretion, information asymmetry, and provability the system creates a nuanced environment where different participants can interact with varying levels of confidence about outcomes, while still maintaining strong guarantees about final state validity. The natural extension to cross-chain messages further amplifies the system's utility by creating a uniform mechanism for handling instructions from both users and other blockchains.
