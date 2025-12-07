# Implementation Guide: Evolving Market Structure with Agent-Based Computational Economics

## Executive Summary

This document provides a comprehensive guide for implementing agent-based computational economics (ACE) models based on Kirman & Vriend (2001) "Evolving Market Structure: An ACE Model of Price Dispersion and Loyalty" and contemporary extensions. It distills core mechanisms, algorithmic specifications, and implementation considerations for recreating and extending this seminal work.

---

## Part 1: Core Theoretical Framework

### 1.1 The Fundamental Puzzle

**Empirical Observations Requiring Explanation:**
- **Price dispersion**: Multiple prices exist simultaneously for homogeneous goods in the same market
- **Buyer loyalty**: Persistent buyer-seller relationships despite no explicit switching costs
- **Coexistence**: Both phenomena persist in equilibrium, not as transient dynamics

**Why Standard Theory Fails:**
- Perfect information available (same market hall, repeated interactions)
- No product differentiation (homogeneous fish, graded by quality)
- No search or switching costs
- No barriers to entry/exit within trading day

**Classical Predictions vs. Reality:**
- **Bertrand competition** → Single price at marginal cost
- **Nash equilibrium** → Law of one price
- **Walrasian equilibrium** → Market clearing at competitive price
- **Reality** → Persistent dispersion (15% coefficient of variation), stable loyalty patterns

### 1.2 Core Insight: Co-evolutionary Emergence

**The Central Mechanism:**

Loyalty and preferential treatment emerge through mutual reinforcement without central coordination or forward-looking optimization:

1. **Buyers learn to be loyal** because:
   - Loyal relationships → higher service rates (97% vs 93% served)
   - Loyal relationships → better prices (on average)
   - Reinforcement learning strengthens strategies that worked

2. **Sellers learn to reward loyalty** because:
   - Loyal customers → higher acceptance rates (92% vs 88%)
   - Loyal customers → more predictable demand
   - Higher transaction completion → higher revenues

3. **Positive feedback loop**:
   - As buyers become more loyal → sellers benefit more from rewarding loyalty
   - As sellers reward loyalty more → buyers benefit more from being loyal
   - System converges to high-loyalty equilibrium

**Why This Is Not Obvious:**

Each individual transaction is a **zero-sum game** for price determination (buyer surplus + seller surplus = constant). However, the **relationship formation game is not zero-sum** because:
- Service rate affects whether transaction occurs at all
- Acceptance rate affects whether price proposal leads to sale
- Both parties can gain by reducing transaction failures

---

## Part 2: Model Architecture

### 2.1 Environment Specification

**Market Structure:**
```
Population:
- 10 sellers (fixed)
- 100 buyers (fixed)
- Duration: 5000 days
- Sessions per day: 2 (morning, afternoon)

Product Characteristics:
- Homogeneous good (fish)
- Indivisible units
- Perishable (cannot be stored)
- Each buyer demands exactly 1 unit/day
```

**Economic Parameters:**
```
Sellers:
- Purchase price (outside market): p_in = 9
- Maximum supply: 30 units
- Pricing range: [0, 20] (discrete integers)

Buyers (base model):
- Resale price (outside market): p_out = 15
- Maximum visits: 1 per session (2 per day)

Buyers (heterogeneous variant):
- Type 1 (33 buyers): p_out = 12
- Type 2 (34 buyers): p_out = 15
- Type 3 (33 buyers): p_out = 18
```

**Temporal Structure:**
```
Each day proceeds as follows:

MORNING:
1. Sellers buy supply outside market at p_in
2. Buyers simultaneously choose seller queues
3. Sellers handle queues (decide order, set prices)
4. Transactions occur or buyers are rejected/denied service
5. Morning session ends

AFTERNOON:
6. Unsatisfied buyers choose seller queues
7. Sellers handle remaining queues
8. Transactions occur or buyers are rejected/denied service
9. Unsold stock perishes
10. Agents update learning rules based on day's outcomes
```

### 2.2 Information Structure

**What Sellers Know:**
- Their own current stock level
- Current queue composition (number waiting)
- Loyalty index L_ij for each buyer i in their queue:
  ```
  L_ij(t) = Σ(x=1 to t) [r_ij(t-x) / (1+α)^(t-x)]

  where:
  r_ij(t-x) = α if buyer i visited seller j on day x
  r_ij(t-x) = 0 otherwise
  α ∈ (0,1) is decay parameter
  ```
- Own past prices, sales, and revenues
- **Do NOT know**: buyer identities, buyer types, buyers' p_out values, other sellers' actions

**What Buyers Know:**
- Prices offered to them by sellers they visit
- Their own transaction history (which sellers visited, outcomes)
- **Do NOT know**: other buyers' experiences, sellers' costs or stocks, prices offered to others

**Critical Asymmetries:**
- Sellers observe buyer loyalty (L_ij) but buyers don't track their own loyalty
- Buyers know their own p_out but sellers don't observe buyer types
- Neither side has market-wide price information
- No communication between agents of the same type

### 2.3 Decision Problems

**Buyers Face 4 Decision Problems:**

1. **Morning seller choice**: Which seller to visit?
   - State: Beginning of morning session
   - Action space: {Seller 1, Seller 2, ..., Seller 10}
   - Information: Own history only

2. **Morning price acceptance**: Accept or reject price offered?
   - State: Seller has proposed price p
   - Action space: {Accept, Reject} for each p ∈ [0, 20]
   - Information: Current price only

3. **Afternoon seller choice** (if unsatisfied): Which seller to visit?
   - State: No transaction in morning
   - Action space: {Seller 1, ..., Seller 10} if open, else ∅
   - Information: Morning outcome, sellers still open

4. **Afternoon price acceptance**: Accept or reject?
   - State: Seller has proposed price p
   - Action space: {Accept, Reject} for each p ∈ [0, 20]
   - Information: Current price, morning rejection price

**Sellers Face 4 Decision Problems:**

1. **Supply quantity**: How much to buy?
   - State: Beginning of day (before market opens)
   - Action space: {0, 1, 2, ..., 30} units
   - Information: Historical sales and profits

2. **Queue handling parameter β**: Advantage/disadvantage to loyal customers?
   - State: Queue has formed with known loyalty values
   - Action space: β ∈ {-25, -20, ..., 0, ..., 20, 25}
   - Mechanism: Customer i selected with probability ∝ (1 + L_ij)^β

3. **Morning pricing**: What price to ask each customer?
   - State: Customer with loyalty class (low/med/high), stock/queue ratio class
   - Action space: p ∈ {0, 1, ..., 20}
   - Information: Loyalty L_ij, current stock, queue length

4. **Afternoon pricing**: What price to ask each customer?
   - State: Same as morning but potentially different stock/queue
   - Action space: p ∈ {0, 1, ..., 20}
   - Information: Same as morning

---

## Part 3: Learning Mechanism (Reinforcement Learning)

### 3.1 Classifier System Architecture

**Core Concept:**
Each decision problem is modeled as a separate classifier system - a collection of condition-action rules with associated strengths (fitness values).

**Classifier System Structure:**
```
Rule Format:
  IF [condition] THEN [action]
  Strength: s_j(t)

Components:
- Condition: State description (may be empty for unconditional rules)
- Action: Specific choice from action space
- Strength: Real-valued measure of rule quality (initially 1.0 for all)
```

**Example Buyer Rules for Seller Choice:**
```
Morning Seller Choice (unconditional):
  IF [always] THEN [choose Seller 1]  s=1.00
  IF [always] THEN [choose Seller 2]  s=1.00
  ...
  IF [always] THEN [choose Seller 10] s=1.00
```

**Example Seller Rules for Pricing:**
```
Morning Pricing (conditional on loyalty and stock/queue ratio):
  IF [loyalty=low, ratio=low] THEN [price=0]   s=1.00
  IF [loyalty=low, ratio=low] THEN [price=1]   s=1.00
  ...
  IF [loyalty=high, ratio=high] THEN [price=20] s=1.00

Total rules: 3 loyalty classes × 3 ratio classes × 21 prices = 189 rules
```

### 3.2 Rule Selection (Stochastic Auction)

**Selection Process Each Period:**

```
ALGORITHM: Stochastic Auction for Rule Selection

INPUT:
  - rules: Set of applicable rules
  - noise_std: Standard deviation for exploration (σ = 0.10)
  - tremble_prob: Probability of random selection (p = 0.025)

OUTPUT:
  - selected_rule: The rule chosen to be active

PROCEDURE:
1. Identify applicable rules matching current state
   applicable_rules ← FILTER(rules, matches_current_state)

2. Normalize strengths to [0,1]
   s_min ← MIN(rule.strength for rule in applicable_rules)
   s_max ← MAX(rule.strength for rule in applicable_rules)

   FOR EACH rule in applicable_rules:
       IF s_max > s_min THEN
           s_norm[rule] ← (rule.strength - s_min) / (s_max - s_min)
       ELSE
           s_norm[rule] ← 0.5
       END IF
   END FOR

3. Add exploration noise and compute bids
   FOR EACH rule in applicable_rules:
       IF RANDOM() < tremble_prob THEN
           bid[rule] ← RANDOM()  // Trembling hand
       ELSE
           ε ← NORMAL(mean=0, std=noise_std)
           bid[rule] ← s_norm[rule] + ε
       END IF
   END FOR

4. Select highest bid
   selected_rule ← ARGMAX(bid)

5. RETURN selected_rule
```

**Rationale:**
- **Exploitation**: Higher strength rules bid higher (on average)
- **Exploration**: Noise and trembling allow testing inferior rules
- **Balance**: σ and p_tremble parameters control exploration rate

### 3.3 Strength Update (Reinforcement)

**Update Rule (Applied once per day after afternoon session):**

```
ALGORITHM: Strength Update

INPUT:
  - s_j(t-1): Current strength of rule j
  - π(t-1): Reward received when rule j was active
  - c: Learning rate (typically 0.05)

OUTPUT:
  - s_j(t): Updated strength

PROCEDURE:
  s_j(t) ← s_j(t-1) - c·s_j(t-1) + c·π(t-1)
  s_j(t) ← (1-c)·s_j(t-1) + c·π(t-1)

  RETURN s_j(t)
```

**Convergence Property:**
```
As t → ∞:
s_j(t) → weighted average of rewards generated by rule j

Specifically:
s_j(∞) ≈ Σ(τ) [(1-c)^τ · π_j(t-τ)]
```

**Interpretation:**
- Strength converges to exponentially-weighted moving average of payoffs
- More recent experiences weighted more heavily
- Learning rate c controls speed of adaptation vs. stability

### 3.4 Reward Functions

**Buyer Rewards:**

```
ALGORITHM: Buyer Reward Computation

Morning Seller Choice:
  IF transaction_occurred THEN
      utility ← p_out - price_paid
      reward ← MAX(0, utility)  // Never negative
  ELSE IF price_rejected THEN
      reward ← 0  // Determined after afternoon outcome
  ELSE IF denied_service THEN
      reward ← 0  // Empty shelves
  END IF

Morning Price Acceptance:
  IF accepted THEN
      reward ← p_out - price_paid
      // Can be negative if buyer accepted unprofitable price
  END IF

  IF rejected_morning AND transacted_afternoon THEN
      reward_for_rejection ← MAX(0, p_out - price_afternoon)
      // Rejection credited only if led to better outcome
  END IF

  IF rejected_morning AND rejected_afternoon THEN
      reward_for_rejection ← 0
      // Rejection led nowhere
  END IF

Afternoon Seller Choice:
  IF transaction_occurred THEN
      utility ← p_out - price_paid
      reward ← MAX(0, utility)
  ELSE IF denied_service THEN
      reward ← 0
  END IF

Afternoon Price Acceptance:
  IF accepted THEN
      reward ← p_out - price_paid
  ELSE IF rejected THEN
      reward ← 0
  END IF
```

**Key Design Choices:**
- Buyers never blame seller choice for bad price acceptance decisions
- Opportunity cost of rejecting morning price includes afternoon outcome
- Service denial doesn't penalize seller choice (seller may have sold out legitimately)

**Seller Rewards:**

```
ALGORITHM: Seller Reward Computation

Supply Quantity:
  net_profit ← gross_revenue - (supply_quantity × p_in)

  // Normalize to [0,1] using last 200 days
  profit_history ← APPEND(profit_history, net_profit)
  recent_profits ← LAST_N(profit_history, 200)

  min_profit ← MIN(recent_profits)
  max_profit ← MAX(recent_profits)

  IF max_profit > min_profit THEN
      reward ← (net_profit - min_profit) / (max_profit - min_profit)
  ELSE
      reward ← 0.5
  END IF

Queue Handling Parameter β:
  max_possible_revenue ← max_price × supply_quantity
  IF max_possible_revenue > 0 THEN
      scaled_revenue ← gross_revenue / max_possible_revenue
      reward ← scaled_revenue  // Already in [0,1]
  ELSE
      reward ← 0
  END IF

Pricing (Morning and Afternoon):
  // For each rule used k times during session
  total_revenue_from_rule ← SUM(accepted_prices_using_this_rule)

  IF times_rule_used > 0 THEN
      average_revenue ← total_revenue_from_rule / times_rule_used
      reward ← average_revenue / max_price  // Normalize to [0,1]
  ELSE
      reward ← 0
  END IF
```

**Key Design Choices:**
- Supply uses net profit (accounts for costs)
- Queue handling uses gross revenue (supply costs already sunk)
- Pricing uses average revenue per proposal (not per acceptance)
- All rewards normalized to [0,1] for comparability

---

## Part 4: Key Mechanisms in Detail

### 4.1 Loyalty Formation

**Loyalty Index L_ij(t):**

```
ALGORITHM: Update Loyalty

INPUT:
  - L_ij_previous: Previous loyalty value for buyer i to seller j
  - visited_today: Boolean, did buyer i visit seller j today?
  - α: Decay parameter (0 < α < 1), typically 0.25

OUTPUT:
  - L_ij_new: Updated loyalty value

PROCEDURE:
  IF visited_today THEN
      r ← α
  ELSE
      r ← 0
  END IF

  // Decay previous loyalty and add today's contribution
  L_ij_new ← L_ij_previous / (1 + α) + r

  RETURN L_ij_new
```

**Properties:**
- **Range**: 0 ≤ L_ij ≤ 1
- **Perfect loyalty**: L_ij = 1 if buyer always visits seller j
- **No visits**: L_ij = 0 if buyer never visits seller j
- **Exponential decay**: Past visits forgotten exponentially
- **Half-life**: ≈ ln(2)/ln(1+α) days

**Example Dynamics:**
```
α = 0.25, buyer visits seller every day:
Day 1: L = 0.25
Day 2: L = 0.25/1.25 + 0.25 = 0.45
Day 3: L = 0.45/1.25 + 0.25 = 0.61
Day 4: L = 0.61/1.25 + 0.25 = 0.74
...
Day ∞: L → 1.00

α = 0.25, buyer visits then stops:
After last visit: L = 0.80
Day 1 after: L = 0.80/1.25 = 0.64
Day 2 after: L = 0.64/1.25 = 0.51
Day 3 after: L = 0.51/1.25 = 0.41
...
```

### 4.2 Queue Handling with Loyalty Preferences

**Selection Mechanism:**

```
ALGORITHM: Select Next Customer from Queue

INPUT:
  - queue: List of buyer IDs waiting
  - loyalty_values: Map from buyer_id → L_ij value
  - β: Treatment parameter (-25 to 25)

OUTPUT:
  - selected_buyer: ID of next buyer to serve

PROCEDURE:
1. Compute weights for each buyer
   weights ← EMPTY_LIST

   FOR EACH buyer_id IN queue:
       L ← loyalty_values[buyer_id]
       weight ← (1 + L)^β
       APPEND(weights, weight)
   END FOR

2. Normalize to probabilities
   total_weight ← SUM(weights)
   probabilities ← EMPTY_LIST

   FOR EACH w IN weights:
       prob ← w / total_weight
       APPEND(probabilities, prob)
   END FOR

3. Random selection with these probabilities
   selected_buyer ← RANDOM_CHOICE(queue, probabilities)

4. RETURN selected_buyer
```

**Effect of β:**
```
β < 0: Loyal customers DISADVANTAGED
  β = -25: Strong preference for NEW customers
  Example: L=0.9 → weight=(1.9)^(-25)=0.000000003
          L=0.1 → weight=(1.1)^(-25)=0.087

β = 0: All customers treated EQUALLY
  All weights = (1+L)^0 = 1

β > 0: Loyal customers ADVANTAGED
  β = +25: Strong preference for LOYAL customers
  Example: L=0.9 → weight=(1.9)^25=1.16×10^7
          L=0.1 → weight=(1.1)^25=11.52
```

**Service Rate Implications:**

Sellers who choose β > 0 will:
- Serve loyal customers earlier in queue
- Loyal customers less likely to arrive late to empty shelves
- Loyal customers experience higher service rate
- BUT: Fewer loyal customers if β too high (they don't develop loyalty without initial service)

### 4.3 Price Determination

**State Discretization for Pricing Rules:**

```
ALGORITHM: Classify Loyalty

INPUT: L_ij (loyalty value between 0 and 1)
OUTPUT: loyalty_class ("low", "medium", or "high")

PROCEDURE:
  IF L_ij < 0.20 THEN
      RETURN "low"
  ELSE IF L_ij < 0.80 THEN
      RETURN "medium"
  ELSE
      RETURN "high"
  END IF

---

ALGORITHM: Classify Stock/Queue Ratio

INPUT:
  - stock: Current inventory
  - queue_length: Number of buyers waiting

OUTPUT: ratio_class ("low", "medium", or "high")

PROCEDURE:
  IF queue_length = 0 THEN
      RETURN "high"  // No queue, plenty of stock
  END IF

  ratio ← stock / queue_length

  IF ratio < 0.75 THEN
      RETURN "low"    // Stock scarce relative to queue
  ELSE IF ratio < 1.25 THEN
      RETURN "medium" // Stock roughly matches queue
  ELSE
      RETURN "high"   // Stock abundant relative to queue
  END IF
```

**Rule Activation:**

```
ALGORITHM: Select Price Rule

INPUT:
  - loyalty_class: "low", "medium", or "high"
  - ratio_class: "low", "medium", or "high"
  - price_rules: Collection of all pricing rules
  - session: "morning" or "afternoon"

OUTPUT:
  - price: Price to charge (0-20)

PROCEDURE:
1. Filter rules matching current state
   applicable_rules ← EMPTY_LIST

   FOR EACH rule IN price_rules:
       IF rule.loyalty = loyalty_class AND rule.ratio = ratio_class THEN
           APPEND(applicable_rules, rule)
       END IF
   END FOR

2. Run stochastic auction among applicable rules
   bids ← EMPTY_LIST

   FOR EACH rule IN applicable_rules:
       // Normalize strength
       strength_norm ← NORMALIZE(rule.strength, applicable_rules)

       // Add noise
       noise ← NORMAL(mean=0, std=0.10)

       // Handle trembling hand
       IF RANDOM() < 0.025 THEN
           bid ← RANDOM()
       ELSE
           bid ← strength_norm + noise
       END IF

       APPEND(bids, (bid, rule.price))
   END FOR

3. Return price of winning rule
   (max_bid, winning_price) ← ARGMAX(bids, key=first_element)

4. RETURN winning_price
```

**Learning Dynamics:**

Sellers learn to charge:
- **Lower prices when**: Stock/queue ratio is low (risk of not selling)
- **Higher prices when**: Stock/queue ratio is high (can afford to be selective)
- **Lower prices to**: Loyal customers (want to maintain relationship)
- **Higher prices to**: Non-loyal customers (less concern about future)

These tendencies emerge from reinforcement - sellers don't reason through this logic.

### 4.4 Co-evolutionary Feedback Loop

**The Positive Feedback Mechanism:**

```
INITIAL STATE (t=0):
- All buyers equally likely to visit all sellers
- All sellers indifferent about loyalty (β≈0)
- Prices roughly uniform

EARLY DYNAMICS (t=1-500):
- Random variation: Some buyers happen to return to same seller
- Some sellers happen to serve returning customers successfully
- These instances get reinforced

EMERGENCE PHASE (t=500-2500):
- Sellers with β>0 get higher revenues from repeat customers
  (higher acceptance rates, more completed transactions)
- These sellers' β>0 rules get strengthened
- Buyers who happened to return to same seller got higher service rates
- These buyers' "return to same seller" rules get strengthened

POSITIVE FEEDBACK:
- More loyal buyers → Sellers benefit more from β>0
- More β>0 by sellers → Buyers benefit more from loyalty
- System accelerates toward high-loyalty state

CONVERGENCE (t>2500):
- ~79% average loyalty (measured by γ_i)
- ~70% of sellers use β>0 (favor loyal customers)
- Mutual reinforcement sustains equilibrium
```

**Why Both Sides Benefit:**

```
Individual transaction: Zero-sum for price splitting

Buyer surplus: p_out - p
Seller surplus: p - p_in
Sum: p_out - p_in (constant, given exogenous p_out and p_in)

---

Relationship formation: Positive-sum via transaction completion

Loyal relationship:
  Service rate: 97%
  Acceptance rate: 92%
  Transaction rate: 0.97 × 0.92 = 89.2%

Non-loyal relationship:
  Service rate: 93%
  Acceptance rate: 88%
  Transaction rate: 0.93 × 0.88 = 81.8%

Efficiency gain from loyalty: 89.2% - 81.8% = 7.4% more trades
```

**Mathematical Representation:**

```
Buyer expected payoff from choosing seller j:
E[π_buyer | j] = P(served) × P(accept|served) × E[p_out - p | accept]

Seller expected payoff from buyer i in queue:
E[π_seller | i] = P(accept | propose) × E[p - p_in | accept]

Co-evolution:
dL_ij/dt = f(E[π_buyer | j] - E[π_buyer | other sellers])
dβ_j/dt = g(E[π_seller | loyal buyers] - E[π_seller | non-loyal buyers])

Equilibrium when:
E[π_buyer | loyal] > E[π_buyer | switch] → Buyers stay loyal
E[π_seller | loyal] > E[π_seller | non-loyal] → Sellers reward loyalty
```

---

## Part 5: Emergent Properties and Results

### 5.1 Price Dynamics

**Temporal Evolution:**

```
Phase 1 (Days 1-500): Exploration and Initial Learning
- Prices start around 10-11 (middle of possible range)
- High variance as agents explore
- Prices asked initially exceed prices accepted
- Coefficient of variation: ~12%

Phase 2 (Days 500-1500): Price Discovery
- Sellers try pushing prices toward p_out-1 = 14
- Some buyers accept high prices initially
- But acceptance rates fall, forcing prices down
- Prices asked and accepted begin converging

Phase 3 (Days 1500-2500): Convergence
- Prices asked approach prices accepted
- Both series trend downward slightly
- Morning avg ≈ 10.3, Afternoon avg ≈ 11.2
- Coefficient of variation: ~8%

Phase 4 (Days 2500-5000): Quasi-Steady State
- Prices relatively stable
- Morning avg: 10.3 (std 0.3)
- Afternoon avg: 11.2 (std 0.6)
- Weak downward trend (would reach 10.0 in another 5000 days)
- Coefficient of variation: ~6%
```

**Distribution at Steady State (Days 2500-5000):**

```
Price    Frequency
  9      14.5%    ← Competitive equilibrium
 10      28.8%    ← Modal price
 11      49.4%    ← Most common
 12       3.7%
13-15     3.6%
16-20     <1%

Mean: 10.3
Median: 11
Mode: 11
Standard deviation: 0.8
Coefficient of variation: 7.8%
```

**Why Not Competitive Equilibrium (p=9)?**

Classical prediction: p* = p_in = 9 (competitive equilibrium)

Reasons for p > 9:
1. **Market power**: Each seller faces queue of buyers, can charge above marginal cost
2. **Imperfect competition**: Not Bertrand (buyers don't observe all prices)
3. **Service differentiation**: Sellers compete on service rate, not just price
4. **Relationship value**: Loyal buyers willing to pay premium for reliable service

**Why Not Game-Theoretic Prediction (p=14)?**

Subgame-perfect equilibrium of ultimatum game: p = p_out - 1 = 14

Reasons for p < 14:
1. **Rejection hurts proposer more**: Seller loses entire margin, buyer just tries another seller
2. **Learning asymmetry**: Feedback from rejection stronger for sellers than buyers
3. **Fairness concerns**: Even non-sophisticated learners avoid extremely unequal splits
4. **Competition**: Multiple sellers creates outside options for buyers

**Morning vs. Afternoon Prices:**

```
Morning:  avg = 10.3
Afternoon: avg = 11.2
Difference: 0.9 (8.7% premium)
```

Why afternoon prices higher?
1. **Selection effect**: Buyers who rejected morning prices are price-sensitive, but those in afternoon market are desperate
2. **Reduced competition**: Some sellers sold out, fewer options
3. **Ultimatum character**: Afternoon is last chance, stronger bargaining position for sellers
4. **Population mix**: Low p_out buyers disproportionately bought in morning

### 5.2 Loyalty Patterns

**Aggregate Loyalty Evolution:**

```
Loyalty index γ_i for buyer i:

γ_i(t) = Σ_j [L_ij(t)]² / [Σ_j L_ij(t)]²

Interpretation:
γ_i = 1.0  → Perfect loyalty (visits only one seller)
γ_i = 0.1  → "Avoiding" behavior (cycles through all 10 sellers)
```

**Temporal Dynamics:**

```
Days 1-500:
  Mean γ: 0.15 → 0.40
  5th percentile: 0.12 → 0.20
  95th percentile: 0.20 → 0.95
  Interpretation: Loyalty begins emerging

Days 500-2500:
  Mean γ: 0.40 → 0.79
  5th percentile: 0.20 → 0.45
  95th percentile: 0.95 → 1.00
  Interpretation: Loyalty strengthens substantially

Days 2500-5000:
  Mean γ: 0.79 ± 0.03 (stable)
  5th percentile: 0.45 ± 0.05
  95th percentile: 1.00
  Interpretation: Quasi-equilibrium reached
```

**Alternative Loyalty Measure:**

```
% of buyers with >95% of monthly purchases from single seller:
  Days 1-25: 8%
  Days 2476-2500: 41%
  Days 4976-5000: 41%

Comparison to real Marseille fish market: 25-50% (depending on fish type)
```

**Heterogeneity in Loyalty:**

Despite identical initial conditions, buyers diverge dramatically:
- ~40% become highly loyal (γ > 0.90)
- ~40% moderately loyal (0.60 < γ < 0.90)
- ~20% remain relatively non-loyal (γ < 0.60)

This heterogeneity arises from:
1. **Path dependence**: Early random experiences get locked in
2. **Seller heterogeneity**: Different sellers offer different service rates
3. **Coordination**: Some buyer-seller pairs "find" each other and stabilize

### 5.3 Service Differentiation

**Service Rate by Customer Type:**

```
Repeat customers (visited same seller previous day):
  Service rate: 97.0%
  Late arrival (empty shelves): 3.0%

Switching customers (visited different seller):
  Service rate: 93.0%
  Late arrival: 7.0%

Relative advantage: 4.0 percentage points
```

**Mechanism Behind Service Differentiation:**

```
Sellers learn β>0 values:
Modal β: +5 to +10
% sellers with β>0: ~70%
% sellers with β=0: ~20%
% sellers with β<0: ~10%

Effect on queue position:
Example queue: [Buyer A (L=0.9), Buyer B (L=0.2), Buyer C (L=0.5)]

β = 0 (no preference):
  Probabilities: [33.3%, 33.3%, 33.3%]

β = +10:
  Weights: [(1.9)^10=613, (1.2)^10=6.2, (1.5)^10=57.7]
  Probabilities: [90.5%, 0.9%, 8.5%]
  → Loyal buyer A served first with 90.5% probability

Result: Loyal buyers served earlier → less likely to encounter empty shelves
```

**Why Service Differentiation Emerges:**

Sellers don't consciously decide "I should reward loyalty." Instead:

1. Some sellers randomly try β>0
2. These sellers experience higher acceptance rates from served customers (loyal customers more likely to accept because they value relationship)
3. Higher acceptance → higher revenue
4. β>0 rules get reinforced
5. Over time, β>0 becomes dominant strategy

### 5.4 Relationship-Specific Outcomes

**Buyer Perspective (Days 2500-5000):**

```
When returning to same seller vs. switching:

  Returning:
    Avg payoff: 4.50
    Service rate: 97%
    Avg price if served: 10.3

  Switching:
    Avg payoff: 4.25
    Service rate: 93%
    Avg price if served: 10.3

  Advantage of loyalty: +0.25 per interaction (5.6%)
```

74% of buyers experience higher average payoff from loyalty.

**Seller Perspective (Days 2500-5000):**

```
When dealing with repeat customer vs. newcomer:

  Repeat customer:
    Avg gross revenue: 9.50 per proposal
    Acceptance rate: 92%
    Effective revenue: 0.92 × 10.35 = 9.52

  Newcomer:
    Avg gross revenue: 9.09 per proposal
    Acceptance rate: 88%
    Effective revenue: 0.88 × 10.31 = 9.07

  Advantage of repeat customers: +0.45 per interaction (5.0%)
```

All sellers (100%) experience higher gross revenue from repeat customers.

**Reconciliation of Zero-Sum Pricing:**

```
Per-transaction surplus split (when transaction occurs):
  Repeat customers: Buyer gets 4.65, Seller gets 10.35 (total: 15.00)
  Switching customers: Buyer gets 4.69, Seller gets 10.31 (total: 15.00)

Switching customers get slightly better prices! So why are loyal relationships beneficial?

Per-interaction expected value (including service/acceptance failures):
  Repeat customers: Buyer gets 4.50, Seller gets 9.50 (total: 14.00)
  Switching customers: Buyer gets 4.25, Seller gets 9.09 (total: 13.34)

Loyalty creates +0.66 in total surplus by increasing transaction completion rate.
```

**The Key Insight:**

Loyalty is valuable not because loyal customers get better prices (they don't, really), but because:
1. Loyal customers more likely to be served (higher service rate)
2. Loyal customers more likely to accept prices (relationship value)
3. More completed transactions = more surplus created
4. Both parties share this efficiency gain

---

## Part 6: Extensions - Heterogeneous Buyers

### 6.1 Three Buyer Types

**Setup:**
```
Type 1: 33 buyers with p_out = 12 (low-value)
Type 2: 34 buyers with p_out = 15 (medium-value)
Type 3: 33 buyers with p_out = 18 (high-value)
```

**Key Question:**

Sellers cannot observe buyer types directly. Can market dynamics lead to:
- Price discrimination?
- Service differentiation?
- Endogenous market segmentation?

### 6.2 Emergent Price Discrimination

**Prices Found and Accepted (Days 2500-5000):**

```
                        Type 1    Type 2    Type 3
                        (p=12)    (p=15)    (p=18)
----------------------------------------------------------------
Morning price accepted:  9.34      9.61      9.78
  Premium over Type 1:    —       +2.9%     +4.8%

Afternoon price accepted: 9.74     10.86     11.20
  Premium over Type 1:    —       +11.5%    +14.9%

Morning price found:     9.41      9.64      9.82
  Premium over Type 1:    —       +2.5%     +4.3%

Afternoon price found:   11.30     11.67     11.71
  Premium over Type 1:    —       +3.3%     +3.6%
```

**Key Findings:**

1. **Systematic price discrimination**: Higher p_out buyers pay more
2. **Larger in afternoon**: When ultimatum character stronger
3. **Even prices "found"**: Not just acceptance decisions, but prices offered
4. **Sellers don't know types**: Discrimination emerges from behavioral differences

### 6.3 Shopping Behavior Differences

**Temporal Patterns:**

```
Morning session outcomes:
                    Type 1    Type 2    Type 3
                    (p=12)    (p=15)    (p=18)
--------------------------------------------------
Purchase            80.6%     90.1%     91.9%
Reject price        13.4%      5.1%      4.1%
Arrive late          6.0%      4.8%      4.0%

Interpretation: High-value buyers more likely to transact in morning
```

**Afternoon Session Population:**

```
Composition of buyers still in market during afternoon:
  Type 1 (p=12): 51.5% (vs. 33% overall)
  Type 2 (p=15): 27.0% (vs. 34% overall)
  Type 3 (p=18): 21.5% (vs. 33% overall)

Interpretation: Afternoon market disproportionately low-value buyers
```

This explains higher afternoon prices - not higher for same buyer, but different buyer composition.

**Loyalty Patterns:**

```
Measure                     Type 1    Type 2    Type 3
                            (p=12)    (p=15)    (p=18)
------------------------------------------------------------
Avg L_ij (loyalty to seller)  0.96      0.93      0.92
  Interpretation: Low-value buyers MORE loyal to specific sellers

Frequency switching seller   7.8%      8.2%      9.1%
  Interpretation: Low-value buyers switch LESS often

Avg γ_i (concentration)      0.75      0.82      0.83
  Interpretation: High-value buyers MORE concentrated overall

Avg # sellers visited        7.4       5.7       6.0
  Interpretation: Low-value buyers visit MORE sellers over time
```

**Apparent Paradox:**

- Type 1 buyers have HIGHER L_ij (more loyal per seller visited)
- Type 1 buyers have LOWER γ_i (less concentrated overall)
- Type 1 buyers visit MORE sellers in total

**Resolution:**

Type 1 buyers: "Serial monogamy"
- Visit fewer sellers at a time (fewer per month)
- Switch between "favorite" sellers more often over time
- When visiting a seller, very loyal to that seller
- Sample more sellers over full 5000 days

Type 3 buyers: "Stable portfolio"
- Visit more sellers per month
- But stick with same set of sellers over time
- Less loyal to any one seller at a time
- Sample fewer sellers over full 5000 days

### 6.4 Service Differentiation by Type

**Service Rates (Days 2500-5000):**

```
                              Type 1    Type 2    Type 3
                              (p=12)    (p=15)    (p=18)
--------------------------------------------------------------
Late - repeat customers        5.8%      4.5%      3.8%
Late - switching customers    10.9%      8.7%      7.8%

Within-type loyalty premium   -5.1pp    -4.2pp    -4.0pp
Across-type effect           -2.0pp    -1.3pp     —
```

**Interpretation:**

1. Within each type: Loyal customers get better service (4-5pp advantage)
2. Across types: High-value customers get better service even when switching
3. Combined effect: Type 3 repeat customer served 97% vs. Type 1 switching customer served 89%

**Mechanism:**

High-value buyers:
- Accept prices more readily → higher acceptance rate observed by sellers
- Complete transactions more often → reinforce sellers' rules
- Sellers learn (implicitly) that buyers with certain behavioral patterns are valuable
- These patterns correlate with buyer type even though sellers don't observe type

### 6.5 Endogenous Seller Specialization

**Seller Customer Composition (Days 2500-5000):**

```
Seller  Type1  Type2  Type3  Total  Specialization
----------------------------------------------------------
1       3.2    2.1    1.4     6.7    Type 1
2       14.1   3.7    1.0    18.8    Type 1
3       9.4    4.4    0.9    14.7    Type 1
4       0.2    0.3    0.1     0.6    Failed
5       3.4    3.8    5.9    13.1    Type 3
6       1.2    3.7    5.3    10.2    Type 3
7       1.1    4.8    3.2     9.1    Type 2
8       0.3    0.7    0.5     1.5    Small
9       0.1    0.3    1.2     1.6    Small
10      10.2   11.0   9.8    31.0    Diversified

Total visits per type: ~43/type on average (100 buyers, 2 sessions)
Actual: Type1≈43, Type2≈35, Type3≈29 (high types transact morning)
```

**Seller Strategies:**

```
Seller 3 (Type 1 specialist):
- Low prices (avg 9.3)
- Low supply/sales ratio (0.82)
- β < 0 (slightly disadvantage loyal customers)
- Low service rate (91%)
- Attracts price-sensitive, low-value buyers

Sellers 5-6 (Type 3 specialists):
- Higher prices (avg 10.1)
- High supply/sales ratio (1.15)
- β > 0 (strongly favor loyal customers)
- High service rate (96%)
- Attract buyers who value reliability over price

Seller 10 (Generalist):
- Medium prices (avg 9.7)
- Medium supply/sales ratio (0.95)
- β ≈ 0 (neutral on loyalty)
- Medium service rate (93%)
- Serves all types roughly equally
```

**Emergence of Specialization:**

Sellers don't decide "I'll target high-value buyers." Instead:

1. **Random variation**: Some sellers happen to serve more Type 3 buyers early
2. **Reinforcement**: High acceptance rates from these buyers strengthen current strategies
3. **Lock-in**: As seller develops high β and high service rate, attracts more Type 3
4. **Differentiation**: Other sellers develop different strategies suited to different types
5. **Equilibrium**: Each seller finds niche in market ecosystem

### 6.6 Correlation Between Loyalty and Price

**Aggregate Relationship:**

```
Correlation(γ_i, price_paid): +0.26 (t-stat: 2.71)

Interpretation: More loyal buyers pay HIGHER prices on average
```

**Mechanism:**

```
High-value buyers (Type 3):
→ Accept prices more readily
→ Develop stable seller relationships (high γ)
→ Sellers learn these patterns predict high acceptance
→ Sellers charge slightly more to buyers with these patterns
→ Creates positive correlation between loyalty and price

But causation is complex:
- NOT: Loyalty → higher prices (exploitation)
- NOT: Higher prices → loyalty (irrationality)
- INSTEAD: Buyer type → both loyalty AND price acceptance
```

**Individual-Level Heterogeneity:**

Within buyer type, loyalty still beneficial:
```
Type 1 buyers:
  High loyalty (γ>0.8): Pay avg 9.42, service rate 96%
  Low loyalty (γ<0.6): Pay avg 9.32, service rate 88%

Type 3 buyers:
  High loyalty (γ>0.8): Pay avg 9.86, service rate 98%
  Low loyalty (γ<0.6): Pay avg 9.76, service rate 94%
```

Loyal buyers pay slightly more but get much better service, net positive.

---

## Part 7: Implementation Guidance

### 7.1 Core Algorithm Structure

```
ALGORITHM: Main Simulation Loop

INPUT:
  - n_sellers: Number of sellers (default 10)
  - n_buyers: Number of buyers (default 100)
  - n_days: Number of days to simulate (default 5000)
  - buyer_types: Array of p_out values for each buyer (or NULL for homogeneous)
  - p_in: Seller purchase price (default 9)

OUTPUT:
  - Simulation results data

PROCEDURE:

1. Initialize agents
   sellers ← CREATE_ARRAY(n_sellers) of Seller objects with p_in

   IF buyer_types IS NULL THEN
       buyers ← CREATE_ARRAY(n_buyers) of Buyer objects with p_out=15
   ELSE
       buyers ← CREATE_ARRAY(n_buyers) of Buyer objects
       FOR i FROM 0 TO n_buyers-1:
           buyers[i].p_out ← buyer_types[i]
       END FOR
   END IF

2. Initialize loyalty matrix L_ij
   loyalty ← MATRIX(n_buyers, n_sellers) filled with zeros

3. Main simulation loop
   FOR day FROM 0 TO n_days-1:

       // ===== MORNING SESSION =====

       // Sellers decide supply
       FOR EACH seller IN sellers:
           seller.CHOOSE_SUPPLY()
       END FOR

       // Buyers choose sellers
       morning_queues ← MAP(seller_id → empty list)
       FOR EACH buyer IN buyers:
           chosen_seller ← buyer.CHOOSE_SELLER(session="morning")
           APPEND(morning_queues[chosen_seller], buyer.id)
       END FOR

       // Sellers handle morning queues
       morning_transactions ← EMPTY_LIST
       FOR EACH seller IN sellers:
           seller.CHOOSE_BETA()  // Queue handling parameter

           queue ← morning_queues[seller.id]
           seller_loyalty ← MAP(buyer_id → loyalty[buyer_id, seller.id])

           transactions ← seller.HANDLE_QUEUE(queue, seller_loyalty, "morning")
           APPEND_ALL(morning_transactions, transactions)
       END FOR

       // ===== AFTERNOON SESSION =====

       // Unsatisfied buyers choose sellers
       afternoon_queues ← MAP(seller_id → empty list)
       FOR EACH buyer IN buyers:
           IF NOT buyer.transacted_morning THEN
               chosen_seller ← buyer.CHOOSE_SELLER(session="afternoon")
               IF chosen_seller IS NOT NULL THEN  // Some sellers may be sold out
                   APPEND(afternoon_queues[chosen_seller], buyer.id)
               END IF
           END IF
       END FOR

       // Sellers handle afternoon queues
       afternoon_transactions ← EMPTY_LIST
       FOR EACH seller IN sellers:
           queue ← afternoon_queues[seller.id]
           IF LENGTH(queue) > 0 AND seller.stock > 0 THEN
               seller_loyalty ← MAP(buyer_id → loyalty[buyer_id, seller.id])

               transactions ← seller.HANDLE_QUEUE(queue, seller_loyalty, "afternoon")
               APPEND_ALL(afternoon_transactions, transactions)
           END IF
       END FOR

       // ===== END OF DAY =====

       // Update loyalty matrix
       FOR EACH buyer IN buyers:
           FOR EACH seller IN sellers:
               visited ← (buyer.visited_morning = seller.id OR
                         buyer.visited_afternoon = seller.id)
               loyalty[buyer.id, seller.id] ← UPDATE_LOYALTY(
                   loyalty[buyer.id, seller.id],
                   visited
               )
           END FOR
       END FOR

       // Reinforcement learning updates
       FOR EACH buyer IN buyers:
           buyer.UPDATE_STRENGTHS()
       END FOR

       FOR EACH seller IN sellers:
           seller.UPDATE_STRENGTHS()
       END FOR

       // Unsold stock perishes
       FOR EACH seller IN sellers:
           seller.stock ← 0
       END FOR

       // Reset daily state
       FOR EACH buyer IN buyers:
           buyer.RESET_DAILY_STATE()
       END FOR
       FOR EACH seller IN sellers:
           seller.RESET_DAILY_STATE()
       END FOR

       // Record data
       RECORD_DAY_DATA(day, buyers, sellers, loyalty,
                      morning_transactions, afternoon_transactions)
   END FOR

4. RETURN results
```

### 7.2 Seller Agent Structure

```
CLASS Seller:

ATTRIBUTES:
  - id: Seller identifier
  - p_in: Purchase price for supply
  - stock: Current inventory
  - beta: Queue handling parameter

  // Classifier systems for each decision
  - supply_rules: Array of supply decision rules
  - beta_rules: Array of queue handling rules
  - price_rules_morning: Array of pricing rules for morning
  - price_rules_afternoon: Array of pricing rules for afternoon

  // Daily tracking
  - gross_revenue: Revenue for current day
  - net_profit: Profit for current day
  - profit_history: Array of past profits (for normalization)

CONSTRUCTOR(seller_id, p_in):
  INITIALIZE all attributes
  supply_rules ← INIT_SUPPLY_RULES()
  beta_rules ← INIT_BETA_RULES()
  price_rules_morning ← INIT_PRICE_RULES()
  price_rules_afternoon ← INIT_PRICE_RULES()

METHOD INIT_SUPPLY_RULES():
  rules ← EMPTY_ARRAY
  FOR quantity FROM 0 TO 30:
      rule ← {action: quantity, strength: 1.0}
      APPEND(rules, rule)
  END FOR
  RETURN rules

METHOD INIT_BETA_RULES():
  rules ← EMPTY_ARRAY
  FOR beta FROM -25 TO 25 STEP 5:
      rule ← {action: beta, strength: 1.0}
      APPEND(rules, rule)
  END FOR
  RETURN rules

METHOD INIT_PRICE_RULES():
  rules ← EMPTY_ARRAY
  loyalty_classes ← ['low', 'medium', 'high']
  ratio_classes ← ['low', 'medium', 'high']

  FOR EACH loyalty IN loyalty_classes:
      FOR EACH ratio IN ratio_classes:
          FOR price FROM 0 TO 20:
              rule ← {
                  condition_loyalty: loyalty,
                  condition_ratio: ratio,
                  action: price,
                  strength: 1.0,
                  times_used: 0,
                  revenue_accumulated: 0
              }
              APPEND(rules, rule)
          END FOR
      END FOR
  END FOR
  RETURN rules

METHOD CHOOSE_SUPPLY():
  selected_rule ← STOCHASTIC_AUCTION(supply_rules, 0.10, 0.025)
  this.stock ← selected_rule.action
  this.active_supply_rule ← selected_rule

METHOD CHOOSE_BETA():
  selected_rule ← STOCHASTIC_AUCTION(beta_rules, 0.10, 0.025)
  this.beta ← selected_rule.action
  this.active_beta_rule ← selected_rule

METHOD HANDLE_QUEUE(queue, loyalty_dict, session):
  transactions ← EMPTY_ARRAY
  remaining_queue ← COPY(queue)

  WHILE LENGTH(remaining_queue) > 0 AND this.stock > 0:
      // Select next buyer using loyalty-weighted lottery
      buyer_id ← SELECT_NEXT_BUYER(remaining_queue, loyalty_dict)
      REMOVE(remaining_queue, buyer_id)

      // Determine price to offer
      buyer_loyalty ← loyalty_dict[buyer_id]
      price ← DETERMINE_PRICE(buyer_loyalty, session)

      // Buyer accepts or rejects
      accepted ← buyers[buyer_id].RESPOND_TO_PRICE(price, session)

      IF accepted THEN
          this.stock ← this.stock - 1
          this.gross_revenue ← this.gross_revenue + price

          transaction ← {
              seller: this.id,
              buyer: buyer_id,
              price: price,
              session: session,
              loyalty: buyer_loyalty
          }
          APPEND(transactions, transaction)
      END IF
  END WHILE

  // Remaining buyers in queue denied service
  FOR EACH buyer_id IN remaining_queue:
      buyers[buyer_id].DENIED_SERVICE(session)
  END FOR

  RETURN transactions

METHOD SELECT_NEXT_BUYER(queue, loyalty_dict):
  weights ← EMPTY_ARRAY
  FOR EACH buyer_id IN queue:
      L ← loyalty_dict[buyer_id]
      weight ← (1 + L) ^ this.beta
      APPEND(weights, weight)
  END FOR

  // Normalize to probabilities
  total ← SUM(weights)
  probabilities ← EMPTY_ARRAY
  FOR EACH w IN weights:
      APPEND(probabilities, w / total)
  END FOR

  // Random selection
  selected ← RANDOM_CHOICE(queue, probabilities)
  RETURN selected

METHOD DETERMINE_PRICE(buyer_loyalty, session):
  // Classify state
  loyalty_class ← CLASSIFY_LOYALTY(buyer_loyalty)
  ratio_class ← CLASSIFY_STOCK_QUEUE_RATIO(this.stock, this.current_queue_length)

  // Get applicable rules
  IF session = "morning" THEN
      rules ← this.price_rules_morning
  ELSE
      rules ← this.price_rules_afternoon
  END IF

  applicable ← FILTER(rules,
      WHERE rule.condition_loyalty = loyalty_class
      AND rule.condition_ratio = ratio_class)

  // Stochastic auction
  selected_rule ← STOCHASTIC_AUCTION(applicable, 0.10, 0.025)

  // Track rule usage for later reinforcement
  selected_rule.times_used ← selected_rule.times_used + 1
  APPEND(this.active_price_rules, selected_rule)

  RETURN selected_rule.action

METHOD UPDATE_STRENGTHS():
  c ← 0.05  // Learning rate

  // Supply rule
  reward ← COMPUTE_SUPPLY_REWARD()
  this.active_supply_rule.strength ←
      (1-c) × this.active_supply_rule.strength + c × reward

  // Beta rule
  reward ← COMPUTE_BETA_REWARD()
  this.active_beta_rule.strength ←
      (1-c) × this.active_beta_rule.strength + c × reward

  // Price rules
  FOR EACH rule IN this.active_price_rules:
      reward ← rule.revenue_accumulated / (rule.times_used × 20)
      rule.strength ← (1-c) × rule.strength + c × reward

      // Reset daily tracking
      rule.times_used ← 0
      rule.revenue_accumulated ← 0
  END FOR

METHOD COMPUTE_SUPPLY_REWARD():
  this.net_profit ← this.gross_revenue - (this.stock × this.p_in)
  APPEND(this.profit_history, this.net_profit)

  // Normalize using last 200 days
  recent_profits ← LAST_N(this.profit_history, 200)
  min_profit ← MIN(recent_profits)
  max_profit ← MAX(recent_profits)

  IF max_profit > min_profit THEN
      reward ← (this.net_profit - min_profit) / (max_profit - min_profit)
  ELSE
      reward ← 0.5
  END IF

  RETURN reward

METHOD COMPUTE_BETA_REWARD():
  max_possible_revenue ← 20 × this.initial_stock
  IF max_possible_revenue > 0 THEN
      reward ← this.gross_revenue / max_possible_revenue
  ELSE
      reward ← 0
  END IF
  RETURN reward

END CLASS
```

### 7.3 Buyer Agent Structure

```
CLASS Buyer:

ATTRIBUTES:
  - id: Buyer identifier
  - p_out: Resale price (value of fish)

  // Classifier systems
  - seller_choice_morning: Array of seller choice rules for morning
  - seller_choice_afternoon: Array of seller choice rules for afternoon
  - price_acceptance_morning: Array of price acceptance/rejection rules
  - price_acceptance_afternoon: Array of price acceptance/rejection rules

  // Daily state
  - transacted_morning: Boolean
  - transacted_afternoon: Boolean
  - visited_morning: Seller ID or NULL
  - visited_afternoon: Seller ID or NULL
  - price_morning: Price offered in morning or NULL
  - price_afternoon: Price offered in afternoon or NULL

CONSTRUCTOR(buyer_id, p_out):
  INITIALIZE all attributes
  seller_choice_morning ← INIT_SELLER_CHOICE()
  seller_choice_afternoon ← INIT_SELLER_CHOICE()
  price_acceptance_morning ← INIT_PRICE_ACCEPTANCE()
  price_acceptance_afternoon ← INIT_PRICE_ACCEPTANCE()

METHOD INIT_SELLER_CHOICE():
  rules ← EMPTY_ARRAY
  FOR seller_id FROM 0 TO 9:
      rule ← {action: seller_id, strength: 1.0}
      APPEND(rules, rule)
  END FOR
  RETURN rules

METHOD INIT_PRICE_ACCEPTANCE():
  rules ← EMPTY_ARRAY
  FOR price FROM 0 TO 20:
      FOR decision IN ['accept', 'reject']:
          rule ← {
              condition_price: price,
              action: decision,
              strength: 1.0
          }
          APPEND(rules, rule)
      END FOR
  END FOR
  RETURN rules

METHOD CHOOSE_SELLER(session):
  IF session = "morning" THEN
      rules ← this.seller_choice_morning
  ELSE
      rules ← this.seller_choice_afternoon
      // Filter to sellers still open
      rules ← FILTER(rules, WHERE sellers[rule.action].stock > 0)

      IF LENGTH(rules) = 0 THEN
          RETURN NULL  // All sold out
      END IF
  END IF

  selected_rule ← STOCHASTIC_AUCTION(rules, 0.10, 0.025)
  seller_id ← selected_rule.action

  IF session = "morning" THEN
      this.visited_morning ← seller_id
      this.active_seller_choice_morning ← selected_rule
  ELSE
      this.visited_afternoon ← seller_id
      this.active_seller_choice_afternoon ← selected_rule
  END IF

  RETURN seller_id

METHOD RESPOND_TO_PRICE(price, session):
  IF session = "morning" THEN
      rules ← this.price_acceptance_morning
  ELSE
      rules ← this.price_acceptance_afternoon
  END IF

  // Get applicable rules for this price
  applicable ← FILTER(rules, WHERE rule.condition_price = price)

  selected_rule ← STOCHASTIC_AUCTION(applicable, 0.10, 0.025)
  decision ← selected_rule.action

  IF session = "morning" THEN
      this.price_morning ← price
      this.active_price_rule_morning ← selected_rule
      IF decision = 'accept' THEN
          this.transacted_morning ← TRUE
      END IF
  ELSE
      this.price_afternoon ← price
      this.active_price_rule_afternoon ← selected_rule
      IF decision = 'accept' THEN
          this.transacted_afternoon ← TRUE
      END IF
  END IF

  RETURN (decision = 'accept')

METHOD DENIED_SERVICE(session):
  IF session = "morning" THEN
      this.denied_morning ← TRUE
  ELSE
      this.denied_afternoon ← TRUE
  END IF

METHOD UPDATE_STRENGTHS():
  c ← 0.05  // Learning rate

  // Morning seller choice
  reward ← COMPUTE_SELLER_CHOICE_REWARD("morning")
  this.active_seller_choice_morning.strength ←
      (1-c) × this.active_seller_choice_morning.strength + c × reward

  // Morning price acceptance/rejection
  IF this.transacted_morning THEN
      reward ← this.p_out - this.price_morning
      this.active_price_rule_morning.strength ←
          (1-c) × this.active_price_rule_morning.strength + c × reward
  ELSE IF this.active_price_rule_morning EXISTS THEN
      // Rejected in morning
      IF this.transacted_afternoon THEN
          reward ← MAX(0, this.p_out - this.price_afternoon)
      ELSE
          reward ← 0
      END IF
      this.active_price_rule_morning.strength ←
          (1-c) × this.active_price_rule_morning.strength + c × reward
  END IF

  // Afternoon seller choice
  IF this.active_seller_choice_afternoon EXISTS THEN
      reward ← COMPUTE_SELLER_CHOICE_REWARD("afternoon")
      this.active_seller_choice_afternoon.strength ←
          (1-c) × this.active_seller_choice_afternoon.strength + c × reward
  END IF

  // Afternoon price acceptance/rejection
  IF this.transacted_afternoon THEN
      reward ← this.p_out - this.price_afternoon
      this.active_price_rule_afternoon.strength ←
          (1-c) × this.active_price_rule_afternoon.strength + c × reward
  ELSE IF this.active_price_rule_afternoon EXISTS THEN
      // Rejected in afternoon
      reward ← 0
      this.active_price_rule_afternoon.strength ←
          (1-c) × this.active_price_rule_afternoon.strength + c × reward
  END IF

METHOD COMPUTE_SELLER_CHOICE_REWARD(session):
  IF session = "morning" THEN
      IF this.transacted_morning THEN
          utility ← this.p_out - this.price_morning
          RETURN MAX(0, utility)
      ELSE IF this.denied_morning THEN
          RETURN 0
      ELSE
          // Rejected price, outcome depends on afternoon
          RETURN 0  // Will be determined after full day
      END IF
  ELSE  // afternoon
      IF this.transacted_afternoon THEN
          utility ← this.p_out - this.price_afternoon
          RETURN MAX(0, utility)
      ELSE
          RETURN 0
      END IF
  END IF

END CLASS
```

### 7.4 Utility Functions

```
FUNCTION STOCHASTIC_AUCTION(rules, noise_std, tremble_prob):
  INPUT:
    - rules: Array of rule objects with 'strength' attribute
    - noise_std: Standard deviation of exploration noise (default 0.10)
    - tremble_prob: Probability of random trembling hand (default 0.025)

  OUTPUT:
    - Selected rule object

  PROCEDURE:
    IF LENGTH(rules) = 0 THEN
        RAISE ERROR "No rules provided"
    END IF

    IF LENGTH(rules) = 1 THEN
        RETURN rules[0]
    END IF

    // Normalize strengths to [0,1]
    strengths ← EXTRACT(rules, 'strength')
    min_s ← MIN(strengths)
    max_s ← MAX(strengths)

    IF max_s > min_s THEN
        strengths_norm ← EMPTY_ARRAY
        FOR EACH s IN strengths:
            normalized ← (s - min_s) / (max_s - min_s)
            APPEND(strengths_norm, normalized)
        END FOR
    ELSE
        strengths_norm ← ARRAY_FILLED_WITH(0.5, LENGTH(strengths))
    END IF

    // Compute bids
    bids ← EMPTY_ARRAY
    FOR i FROM 0 TO LENGTH(rules)-1:
        IF RANDOM() < tremble_prob THEN
            // Trembling hand: random bid
            bid ← RANDOM()
        ELSE
            // Normal: strength + noise
            noise ← NORMAL(mean=0, std=noise_std)
            bid ← strengths_norm[i] + noise
        END IF

        APPEND(bids, bid)
    END FOR

    // Select highest bid
    max_bid_idx ← ARGMAX(bids)
    RETURN rules[max_bid_idx]

---

FUNCTION UPDATE_LOYALTY(L_prev, visited_today, alpha):
  INPUT:
    - L_prev: Previous loyalty value
    - visited_today: Boolean
    - alpha: Decay parameter (default 0.25)

  OUTPUT:
    - Updated loyalty value

  PROCEDURE:
    IF visited_today THEN
        r ← alpha
    ELSE
        r ← 0
    END IF

    L_new ← L_prev / (1 + alpha) + r
    RETURN L_new

---

FUNCTION CLASSIFY_LOYALTY(L):
  INPUT: L (loyalty value between 0 and 1)
  OUTPUT: loyalty_class ("low", "medium", or "high")

  PROCEDURE:
    IF L < 0.20 THEN
        RETURN "low"
    ELSE IF L < 0.80 THEN
        RETURN "medium"
    ELSE
        RETURN "high"
    END IF

---

FUNCTION CLASSIFY_STOCK_QUEUE_RATIO(stock, queue_length):
  INPUT:
    - stock: Current inventory
    - queue_length: Number of buyers waiting

  OUTPUT: ratio_class ("low", "medium", or "high")

  PROCEDURE:
    IF queue_length = 0 THEN
        RETURN "high"
    END IF

    ratio ← stock / queue_length

    IF ratio < 0.75 THEN
        RETURN "low"
    ELSE IF ratio < 1.25 THEN
        RETURN "medium"
    ELSE
        RETURN "high"
    END IF

---

FUNCTION COMPUTE_LOYALTY_CONCENTRATION(loyalty_matrix, buyer_id):
  INPUT:
    - loyalty_matrix: 2D array [n_buyers, n_sellers]
    - buyer_id: Buyer index

  OUTPUT:
    - Concentration index γ (0 to 1)

  PROCEDURE:
    L_i ← loyalty_matrix[buyer_id, ALL_COLUMNS]

    numerator ← SUM(L_i^2)
    denominator ← (SUM(L_i))^2

    IF denominator > 0 THEN
        gamma ← numerator / denominator
    ELSE
        gamma ← 1.0 / NUMBER_OF_SELLERS  // Uniform if no visits yet
    END IF

    RETURN gamma
```

### 7.5 Data Collection and Analysis

```
CLASS DataCollector:

ATTRIBUTES:
  - daily_data: Array of daily aggregate statistics
  - transaction_data: Array of individual transactions

METHOD RECORD_DAY(day, buyers, sellers, loyalty, morning_trans, afternoon_trans):
  // Aggregate statistics
  IF LENGTH(morning_trans) > 0 THEN
      avg_price_morning ← MEAN(EXTRACT(morning_trans, 'price'))
  ELSE
      avg_price_morning ← NULL
  END IF

  IF LENGTH(afternoon_trans) > 0 THEN
      avg_price_afternoon ← MEAN(EXTRACT(afternoon_trans, 'price'))
  ELSE
      avg_price_afternoon ← NULL
  END IF

  loyalty_concentrations ← EMPTY_ARRAY
  FOR i FROM 0 TO LENGTH(buyers)-1:
      gamma ← COMPUTE_LOYALTY_CONCENTRATION(loyalty, i)
      APPEND(loyalty_concentrations, gamma)
  END FOR

  day_stats ← {
      day: day,
      avg_price_morning: avg_price_morning,
      avg_price_afternoon: avg_price_afternoon,
      n_transactions_morning: LENGTH(morning_trans),
      n_transactions_afternoon: LENGTH(afternoon_trans),
      avg_loyalty: MEAN(loyalty_concentrations)
  }

  APPEND(this.daily_data, day_stats)

  // Individual transactions
  FOR EACH trans IN (morning_trans + afternoon_trans):
      trans.day ← day
      APPEND(this.transaction_data, trans)
  END FOR

METHOD ANALYZE_STEADY_STATE(start_day, end_day):
  // Filter to steady state period
  transactions ← FILTER(this.transaction_data,
      WHERE start_day ≤ trans.day < end_day)

  daily ← FILTER(this.daily_data,
      WHERE start_day ≤ d.day < end_day)

  // Price distribution
  prices ← EXTRACT(transactions, 'price')
  price_dist ← {
      mean: MEAN(prices),
      median: MEDIAN(prices),
      std: STD(prices),
      mode: MODE(prices),
      histogram: HISTOGRAM(prices, bins=[0,1,2,...,21])
  }

  // Loyalty statistics
  loyalty_values ← EXTRACT(daily, 'avg_loyalty')
  loyalty_stats ← {
      mean: MEAN(loyalty_values),
      std: STD(loyalty_values),
      trajectory: loyalty_values
  }

  RETURN {
      price_distribution: price_dist,
      loyalty: loyalty_stats,
      n_transactions: LENGTH(transactions)
  }

METHOD ANALYZE_BUYER_SELLER_RELATIONSHIPS(loyalty_matrix, buyers, sellers, start_day):
  // For each buyer, compute payoff when returning vs switching
  results ← EMPTY_ARRAY

  FOR buyer_id FROM 0 TO LENGTH(buyers)-1:
      transactions ← FILTER(this.transaction_data,
          WHERE trans.buyer = buyer_id AND trans.day ≥ start_day)

      // Identify repeat vs switching
      FOR i FROM 1 TO LENGTH(transactions)-1:
          prev_seller ← transactions[i-1].seller
          curr_seller ← transactions[i].seller

          repeat ← (prev_seller = curr_seller)

          result ← {
              buyer: buyer_id,
              repeat: repeat,
              price: transactions[i].price,
              payoff: buyers[buyer_id].p_out - transactions[i].price
          }
          APPEND(results, result)
      END FOR
  END FOR

  // Aggregate
  repeat_payoffs ← EXTRACT(FILTER(results, WHERE r.repeat), 'payoff')
  switch_payoffs ← EXTRACT(FILTER(results, WHERE NOT r.repeat), 'payoff')

  RETURN {
      avg_payoff_repeat: MEAN(repeat_payoffs),
      avg_payoff_switch: MEAN(switch_payoffs),
      advantage: MEAN(repeat_payoffs) - MEAN(switch_payoffs)
  }

END CLASS
```

### 7.6 Visualization Pseudo-code

```
FUNCTION PLOT_PRICE_EVOLUTION(data_collector):
  daily ← data_collector.daily_data

  // Moving average for smoothing
  window ← 20
  days ← EXTRACT(daily, 'day')

  prices_morning ← EXTRACT(daily, 'avg_price_morning')
  prices_morning_smooth ← MOVING_AVERAGE(prices_morning, window)

  prices_afternoon ← EXTRACT(daily, 'avg_price_afternoon')
  prices_afternoon_smooth ← MOVING_AVERAGE(prices_afternoon, window)

  CREATE_FIGURE(width=12, height=6)
  PLOT_LINE(days, prices_morning_smooth, label='Morning', linewidth=2)
  PLOT_LINE(days, prices_afternoon_smooth, label='Afternoon', linewidth=2)
  PLOT_HORIZONTAL_LINE(y=9, color='red', linestyle='dashed', label='p_in (competitive)')
  PLOT_HORIZONTAL_LINE(y=14, color='green', linestyle='dashed', label='p_out-1 (game theory)')

  SET_XLABEL('Day')
  SET_YLABEL('Average Price')
  SET_TITLE('Price Evolution Over Time (20-day moving average)')
  ADD_LEGEND()
  ADD_GRID(alpha=0.3)

  SAVE_FIGURE('price_evolution.png', dpi=300)
  SHOW_FIGURE()

---

FUNCTION PLOT_LOYALTY_EVOLUTION(data_collector):
  daily ← data_collector.daily_data

  window ← 20
  days ← EXTRACT(daily, 'day')
  loyalty ← EXTRACT(daily, 'avg_loyalty')
  loyalty_smooth ← MOVING_AVERAGE(loyalty, window)

  CREATE_FIGURE(width=12, height=6)
  PLOT_LINE(days, loyalty_smooth, linewidth=2, color='purple')

  SET_XLABEL('Day')
  SET_YLABEL('Average Loyalty (γ)')
  SET_TITLE('Loyalty Evolution Over Time (20-day moving average)')
  SET_Y_LIMITS([0, 1])
  ADD_GRID(alpha=0.3)

  SAVE_FIGURE('loyalty_evolution.png', dpi=300)
  SHOW_FIGURE()

---

FUNCTION PLOT_PRICE_DISTRIBUTION(data_collector, start_day, end_day):
  transactions ← FILTER(data_collector.transaction_data,
      WHERE start_day ≤ trans.day < end_day)

  prices ← EXTRACT(transactions, 'price')

  CREATE_FIGURE(width=10, height=6)
  PLOT_HISTOGRAM(prices, bins=[0,1,2,...,21],
                 density=TRUE, alpha=0.7,
                 color='skyblue', edgecolor='black')

  SET_XLABEL('Price')
  SET_YLABEL('Relative Frequency')
  SET_TITLE('Price Distribution (Days ' + start_day + '-' + end_day + ')')
  ADD_GRID(alpha=0.3, axis='y')

  SAVE_FIGURE('price_distribution.png', dpi=300)
  SHOW_FIGURE()

---

FUNCTION MOVING_AVERAGE(data, window):
  result ← EMPTY_ARRAY
  FOR i FROM 0 TO LENGTH(data)-1:
      start ← MAX(0, i - window + 1)
      window_data ← data[start:i+1]
      avg ← MEAN(REMOVE_NULL(window_data))
      APPEND(result, avg)
  END FOR
  RETURN result
```

---

## Part 8: Extensions and Variations

### 8.1 Alternative Learning Algorithms

**Possible Variations:**

1. **Experience-weighted attraction (EWA)**
   - Hybrid of reinforcement and belief learning
   - Tracks both actual and hypothetical payoffs
   - Smoother convergence, less exploration

2. **Q-learning**
   - State-action value functions
   - Temporal difference updates
   - More sophisticated credit assignment

3. **Genetic algorithms**
   - Population of strategies per agent
   - Crossover and mutation operators
   - Stronger exploration of strategy space

4. **Aspiration-based learning**
   - Agents have target aspiration levels
   - Satisfied if exceeding aspiration
   - Search more when dissatisfied

### 8.2 Market Structure Variations

**Parameter Variations:**

```
// Increase market size
n_sellers ← 20
n_buyers ← 200

// More sessions per day
n_sessions ← 4  // Morning, midday, afternoon, evening

// Partial perishability
perishability_rate ← 0.5  // 50% spoils, 50% can be sold next day

// Multiple goods
n_goods ← 3
buyer_preferences ← MAP where each buyer wants different bundle

// Entry/exit dynamics
entry_prob ← 0.01  // Per day probability new seller enters
exit_threshold ← -10  // Exit if cumulative profit below threshold
```

### 8.3 Information Structure Variations

**Alternative Assumptions:**

1. **Posted prices**
   - Sellers post prices publicly before queues form
   - Buyers observe all prices before choosing
   - Tests whether price dispersion persists with transparency

2. **Price communication**
   - Buyers can share price information with each other
   - Network of information transmission
   - Tests role of information frictions

3. **Seller identities known**
   - Buyers can identify sellers explicitly
   - Can track seller-specific histories more precisely
   - Tests whether anonymity is important

4. **Type revelation**
   - Buyers' p_out values observable to sellers
   - Perfect price discrimination possible
   - Tests whether implicit type inference is important

### 8.4 Policy Interventions

**Possible Experiments:**

```
1. Price ceiling:
   max_allowed_price ← 12
   // Enforced in seller price selection

2. Mandatory price posting:
   // Sellers must announce single price to all
   // Cannot price discriminate

3. Anti-discrimination regulation:
   // Sellers cannot use β > 0
   // Must serve FIFO

4. Information provision:
   // Buyers shown average market price each day
   // Tests whether information reduces dispersion
```

### 8.5 Comparative Statics

**Questions to Explore:**

1. **Effect of p_out - p_in spread**
   - Narrow spread → more price competition?
   - Wide spread → more scope for dispersion?

2. **Effect of buyer/seller ratio**
   - More buyers → sellers have more power?
   - More sellers → buyers have more options?

3. **Effect of supply constraints**
   - Tighter supply → higher prices, more loyalty?
   - Abundant supply → lower prices, less loyalty?

4. **Effect of learning rate**
   - Faster learning (high c) → quicker convergence?
   - Slower learning (low c) → more persistent experimentation?

---

## Part 9: Validation and Calibration

### 9.1 Stylized Facts to Match

**From Real Marseille Fish Market:**

```
Target empirical moments:

1. Price dispersion
   - Coefficient of variation: 6-8%
   - Range: Typically 85% to 115% of mean price

2. Loyalty
   - ~30-40% buyers >95% purchases from single seller
   - Average loyalty concentration: 0.70-0.85

3. Price-loyalty correlation
   - Slightly positive or neutral
   - Loyal customers don't systematically pay much more/less

4. Service differentiation
   - Observable quality differences across sellers
   - Repeat customers better treatment

5. Market clearing
   - Typically >95% of supply sells
   - <5% of buyers go unsatisfied
```

### 9.2 Calibration Strategy

```
ALGORITHM: Calibrate Parameters

INPUT:
  - target_moments: Map of empirical values to match
  - param_ranges: Map of (min, max) for each parameter

OUTPUT:
  - Best-fit parameters

PROCEDURE:

DEFINE FUNCTION objective(params):
    // Run simulation with these parameters
    results ← RUN_SIMULATION(params)

    // Compute simulated moments
    sim_moments ← COMPUTE_MOMENTS(results)

    // Distance from targets
    distance ← 0
    FOR EACH key IN target_moments:
        target ← target_moments[key]
        simulated ← sim_moments[key]
        distance ← distance + ((simulated - target) / target)^2
    END FOR

    RETURN distance
END FUNCTION

// Optimization (grid search, or more sophisticated)
best_params ← OPTIMIZE(objective, param_ranges)

RETURN best_params

---

// Primary parameters to calibrate
parameters ← {
    alpha: 0.25,              // Loyalty decay rate
    learning_rate: 0.05,      // Strength update rate
    noise_std: 0.10,          // Exploration noise
    tremble_prob: 0.025,      // Random exploration probability
    n_sellers: 10,
    n_buyers: 100,
    p_in: 9,
    p_out: 15
}

// Target moments from real data
empirical_moments ← {
    cv_price: 0.07,           // Coefficient of variation
    avg_loyalty: 0.78,        // Average γ
    pct_high_loyalty: 0.35,   // % with γ > 0.95
    service_rate: 0.96        // % of buyers who transact
}
```

### 9.3 Sensitivity Analysis

```
ALGORITHM: Sensitivity Analysis

INPUT:
  - base_params: Baseline parameter map
  - vary_param: Name of parameter to vary
  - param_range: Array of values to try

OUTPUT:
  - Results for each parameter value

PROCEDURE:
  results ← EMPTY_ARRAY

  FOR EACH value IN param_range:
      params ← COPY(base_params)
      params[vary_param] ← value

      // Run multiple replications
      replications ← EMPTY_ARRAY
      FOR rep FROM 0 TO 9:
          sim_results ← RUN_SIMULATION(params, seed=rep)
          moments ← COMPUTE_MOMENTS(sim_results)
          APPEND(replications, moments)
      END FOR

      // Average across replications
      avg_moments ← AVERAGE_MOMENTS(replications)
      std_moments ← STD_MOMENTS(replications)

      result ← {
          param_value: value,
          moments: avg_moments,
          std: std_moments
      }
      APPEND(results, result)
  END FOR

  RETURN results

---

// Example: sensitivity to learning rate
sensitivity_results ← SENSITIVITY_ANALYSIS(
    base_params: parameters,
    vary_param: 'learning_rate',
    param_range: [0.01, 0.05, 0.10, 0.20, 0.50]
)
```

### 9.4 Out-of-Sample Validation

```
ALGORITHM: Test Shock Response

PROCEDURE:

1. Run baseline for 5000 days
   baseline ← RUN_SIMULATION(n_days=5000)

2. Introduce shock (e.g., change p_out for some buyers)
   shocked_buyers ← RANDOM_SAMPLE(buyers, 30)
   FOR EACH b IN shocked_buyers:
       buyers[b].p_out ← 18
   END FOR

3. Continue for 2000 more days
   shock_response ← CONTINUE_SIMULATION(2000)

4. Model predicts:
   - Shocked buyers will increase loyalty concentration
   - Will pay higher prices
   - Will get better service

5. Compare predictions to simulation outcomes

---

ALGORITHM: Test Intervention

PROCEDURE:

1. Model calibrated without price ceiling
2. Now impose ceiling and check predictions
   intervention ← RUN_SIMULATION_WITH_PRICE_CEILING(
       max_price=12
   )

3. Predictions to test:
   - Price dispersion should decrease
   - Some sellers may exit (unprofitable)
   - Loyalty may decrease (less service differentiation)

4. Compare predictions to simulation outcomes
```

---

## Part 10: Key Takeaways for Implementation

### 10.1 Critical Design Choices

**Most Important Decisions:**

1. **Reward structure design**
   - Never negative rewards for choices (only for outcomes)
   - Buyer seller-choice rewards never penalized for bad price decisions
   - Sellers' pricing rewards based on per-proposal revenue, not per-acceptance
   - These create appropriate credit assignment

2. **State discretization**
   - Loyalty: 3 classes sufficient (low/med/high)
   - Stock/queue ratio: 3 classes sufficient
   - Finer discretization slows learning without improving outcomes

3. **Exploration parameters**
   - Noise std = 0.10 balances exploration/exploitation
   - Tremble prob = 0.025 maintains baseline experimentation
   - Too little exploration → lock into suboptimal patterns
   - Too much exploration → never converge

4. **Learning rate**
   - c = 0.05 balances adaptation speed vs stability
   - Strength update: s(t) = (1-c)·s(t-1) + c·π
   - Lower c → slower learning, more stable
   - Higher c → faster adaptation, more volatile

### 10.2 Common Pitfalls to Avoid

**Mistakes to Watch For:**

```
1. Incorrect credit assignment

   // WRONG: Penalize seller choice if price too high
   IF price > p_out THEN
       reward_seller_choice ← -1  // NO!
   END IF

   // RIGHT: Seller choice only penalized if denied service
   IF denied_service THEN
       reward_seller_choice ← 0
   ELSE
       reward_seller_choice ← MAX(0, p_out - price)
   END IF

---

2. Mixing sessions

   // WRONG: Use same classifier for morning and afternoon
   price_rules ← INIT_PRICE_RULES()  // Shared

   // RIGHT: Separate classifiers for each session
   price_rules_morning ← INIT_PRICE_RULES()
   price_rules_afternoon ← INIT_PRICE_RULES()

---

3. Forgetting to reset state

   // WRONG: Loyalty persists across market closures
   // (Should decay when market not visited)

   // RIGHT: Update loyalty every day for all buyer-seller pairs
   FOR i IN buyers:
       FOR j IN sellers:
           visited ← (i visited j today)
           loyalty[i,j] ← UPDATE_LOYALTY(loyalty[i,j], visited)
       END FOR
   END FOR

---

4. Incorrect normalization

   // WRONG: Normalize strengths globally
   all_strengths ← EXTRACT(all_rules, 'strength')
   min_s ← MIN(all_strengths)
   max_s ← MAX(all_strengths)

   // RIGHT: Normalize within applicable rule set only
   applicable ← FILTER(rules, WHERE rule matches state)
   strengths ← EXTRACT(applicable, 'strength')
   min_s ← MIN(strengths)
   max_s ← MAX(strengths)
```

### 10.3 Performance Optimization

**Computational Efficiency:**

```
1. Vectorize loyalty updates

   // SLOW: Loop through all pairs
   FOR i FROM 0 TO n_buyers-1:
       FOR j FROM 0 TO n_sellers-1:
           loyalty[i,j] ← UPDATE(...)
       END FOR
   END FOR

   // FAST: Vectorized operation (if language supports)
   loyalty ← loyalty / (1 + alpha) + visit_matrix × alpha

---

2. Cache rule lookups

   // SLOW: Filter rules every time
   applicable ← FILTER(rules, WHERE r.loyalty=L_class)

   // FAST: Pre-organize by condition
   rules_by_state ← MAP {
       ('low', 'low'): [...],
       ('low', 'med'): [...],
       // etc.
   }
   applicable ← rules_by_state[(L_class, ratio_class)]

---

3. Batch strength updates

   // SLOW: Update after each transaction
   FOR EACH trans IN transactions:
       UPDATE_STRENGTH(...)
   END FOR

   // FAST: Accumulate rewards, update once per day
   FOR EACH day IN days:
       ACCUMULATE_REWARDS(...)
   END FOR
   UPDATE_ALL_STRENGTHS()
```

### 10.4 Recommended Workflow

**Step-by-Step Implementation:**

```
1. Start with minimal model
   - Homogeneous buyers (p_out = 15)
   - Morning session only
   - Simplified decisions (e.g., no queue handling parameter)

2. Verify basic properties
   - Prices decline from random start
   - Some loyalty emerges
   - Market clears reasonably

3. Add complexity incrementally
   - Add afternoon session
   - Add queue handling parameter β
   - Test after each addition

4. Implement heterogeneity
   - Add multiple buyer types
   - Verify segmentation emerges

5. Calibrate to empirical moments
   - Compare with target moments
   - Adjust parameters systematically

6. Sensitivity analysis
   - Vary parameters around best-fit
   - Check robustness of results

7. Policy experiments
   - Test interventions
   - Generate predictions
```

### 10.5 Debugging Strategies

```
PROCEDURE: Debugging Checks

// Check 1: Are prices reasonable?
ASSERT 0 ≤ mean_price ≤ 20
ASSERT p_in ≤ mean_price ≤ p_out
// If violated: check reward functions

// Check 2: Is market clearing?
ASSERT sell_through_rate > 0.90
// If violated: check supply decisions

// Check 3: Is learning happening?
ASSERT STRENGTH_VARIANCE_DECREASES_OVER_TIME()
// If violated: check learning rate, exploration parameters

// Check 4: Is loyalty developing?
ASSERT LOYALTY_CONCENTRATION_INCREASES()
// If violated: check loyalty update formula, β learning

// Check 5: Are rules being used?
FOR EACH rule IN all_rules:
    ASSERT rule.times_used > 0  // Over long run
END FOR
// If violated: some rules unreachable, check state discretization

// Check 6: Verify stochastic auction
bids ← EMPTY_ARRAY
FOR iteration FROM 1 TO 1000:
    selected ← STOCHASTIC_AUCTION(rules)
    APPEND(bids, selected.id)
END FOR

strongest_rule ← ARGMAX(rules, key='strength')
ASSERT COUNT(bids, strongest_rule.id) > 800  // Should win ~90%+
```

---

## Summary

This document provides comprehensive guidance for implementing the Kirman & Vriend (2001) ACE model of market structure evolution. The key insights are:

1. **Theoretical core**: Co-evolutionary emergence of loyalty and preferential treatment through simple reinforcement learning, without strategic foresight

2. **Mechanism**: Both buyers and sellers benefit from loyal relationships not through price concessions but through higher transaction completion rates

3. **Critical implementation details**:
   - Separate classifier systems for each decision problem
   - Careful reward function design for proper credit assignment
   - Stochastic auction for rule selection with exploration
   - Exponentially-weighted strength updates

4. **Emergent phenomena**:
   - Price dispersion persists despite perfect information
   - Loyalty develops endogenously without switching costs
   - Market segmentation emerges without explicit type recognition
   - Service differentiation arises from relationship value

5. **Extensions**: The base model can be extended to explore heterogeneity, policy interventions, alternative learning algorithms, and market structure variations

The model demonstrates how complex market phenomena emerge from simple adaptive behavior, providing an alternative to equilibrium-based analysis for understanding decentralized markets with learning agents.
