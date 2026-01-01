# Replication Guide: Gode & Sunder (1993)
# "Allocative Efficiency of Markets with Zero-Intelligence Traders"

## Overview

This document provides implementation specifications for replicating the experiments in Gode & Sunder's seminal 1993 paper. The goal is to enable independent verification of their central finding: that double auction markets achieve high allocative efficiency even when populated by traders who submit random bids and offers, provided those traders are constrained from making losing trades.

---

## 1. Market Mechanism: Continuous Double Auction

### 1.1 Core Rules

The market operates as a **continuous double auction (CDA)** with the following properties:

1. **Participants**: 12 traders total, divided into 6 buyers and 6 sellers
2. **Order Types**: Limit orders only (bid or ask with specified price)
3. **Order Size**: Single unit per order (simplification from standard CDA)
4. **Order Book**: Only the best (highest) bid and best (lowest) ask are displayed
5. **Order Lifetime**: Previous unaccepted orders are canceled when a new order arrives
6. **Price Priority**: When a bid ≥ ask, transaction occurs at the price of the earlier order

### 1.2 Transaction Execution

```
FUNCTION process_order(new_order, order_book):
    IF new_order.type == BID:
        IF order_book.best_ask EXISTS AND new_order.price >= order_book.best_ask.price:
            # Transaction occurs
            transaction_price = order_book.best_ask.price  # Earlier order's price
            EXECUTE transaction at transaction_price
            REMOVE order_book.best_ask
            RETURN transaction_record
        ELSE:
            # No cross - update best bid
            order_book.best_bid = new_order
            RETURN null

    IF new_order.type == ASK:
        IF order_book.best_bid EXISTS AND new_order.price <= order_book.best_bid.price:
            # Transaction occurs  
            transaction_price = order_book.best_bid.price  # Earlier order's price
            EXECUTE transaction at transaction_price
            REMOVE order_book.best_bid
            RETURN transaction_record
        ELSE:
            # No cross - update best ask
            order_book.best_ask = new_order
            RETURN null
```

### 1.3 Timing Structure

- **Market Duration**: 6 trading periods per market session
- **Period Duration**:
  - Human traders: 4 minutes per period
  - ZI traders: 30 seconds per period (sufficient for all possible trades)
- **Period Reset**: At each period start, all traders receive fresh unit endowments with identical values/costs

---

## 2. Induced Value Mechanism

### 2.1 Buyer Valuation Structure

Each buyer receives an endowment of units they have the right (but not obligation) to purchase. Each unit has a private **redemption value** known only to that buyer.

```
BUYER PROFIT CALCULATION:
For each unit purchased at price p with redemption value v:
    profit = v - p

Constraint: A rational buyer should never bid above v (would guarantee loss)
```

### 2.2 Seller Cost Structure

Each seller receives an endowment of units they have the right (but not obligation) to sell. Each unit has a private **cost** known only to that seller.

```
SELLER PROFIT CALCULATION:
For each unit sold at price p with cost c:
    profit = p - c

Constraint: A rational seller should never offer below c (would guarantee loss)
```

### 2.3 Unit Trading Sequence Constraint

Traders must trade their units in order: a trader cannot trade unit (i+1) until unit i has been traded. This creates stepped supply and demand curves.

---

## 3. Trader Specifications

### 3.1 Zero-Intelligence Unconstrained (ZI-U)

```
FUNCTION generate_ZI_U_order(trader):
    price = RANDOM_UNIFORM_INTEGER(1, 200)

    IF trader.role == BUYER:
        RETURN Order(type=BID, price=price, trader_id=trader.id)
    ELSE:
        RETURN Order(type=ASK, price=price, trader_id=trader.id)
```

**Properties**:
- No memory of past events
- No learning or adaptation
- No profit-seeking behavior
- Can and will make trades that result in losses
- Uniform distribution over entire price range [1, 200]

### 3.2 Zero-Intelligence Constrained (ZI-C)

```
FUNCTION generate_ZI_C_order(trader):
    current_unit = trader.get_next_tradeable_unit()

    IF current_unit IS NULL:
        RETURN null  # No units left to trade

    IF trader.role == BUYER:
        max_price = current_unit.redemption_value
        price = RANDOM_UNIFORM_INTEGER(1, max_price)
        RETURN Order(type=BID, price=price, trader_id=trader.id)
    ELSE:
        min_price = current_unit.cost
        price = RANDOM_UNIFORM_INTEGER(min_price, 200)
        RETURN Order(type=ASK, price=price, trader_id=trader.id)
```

**Properties**:
- No memory of past events
- No learning or adaptation  
- No profit-seeking behavior
- **Cannot make trades that result in losses** (the key constraint)
- Uniform distribution over feasible price range only
- Feasible range changes as units are traded

### 3.3 Human Traders

In the original experiment, human participants were MBA students who:
- Were motivated by course credit tied to trading profits
- Could observe the current best bid and ask
- Could observe the transaction history
- Made their own strategic decisions about when and at what price to trade

For simulation purposes, human behavior can be approximated by more sophisticated algorithms (ZIP, GD, AA) but the original comparison is specifically against the ZI baselines.

---

## 4. Market Configurations

### 4.1 Market 1

**Equilibrium**: Price ≈ 69, Quantity = medium

| Buyers | Unit 1 Value | Unit 2 Value | Unit 3 Value | Unit 4 Value |
|--------|--------------|--------------|--------------|--------------|
| B1 | 150 | 100 | 70 | 50 |
| B2 | 145 | 95 | 68 | 45 |
| B3 | 140 | 90 | 66 | 40 |
| B4 | 135 | 85 | 64 | 35 |
| B5 | 130 | 80 | 62 | 30 |
| B6 | 125 | 75 | 60 | 25 |

| Sellers | Unit 1 Cost | Unit 2 Cost | Unit 3 Cost | Unit 4 Cost |
|---------|-------------|-------------|-------------|-------------|
| S1 | 10 | 45 | 65 | 95 |
| S2 | 15 | 50 | 67 | 100 |
| S3 | 20 | 55 | 69 | 105 |
| S4 | 25 | 60 | 71 | 110 |
| S5 | 30 | 62 | 73 | 115 |
| S6 | 35 | 64 | 75 | 120 |

*Note: These are representative values reconstructed from the paper's figures. The exact values should be calibrated to produce the reported equilibrium.*

### 4.2 Market 2

**Equilibrium**: Price ≈ 69, Quantity = medium  
**Design Feature**: Different curve shapes than Market 1, same equilibrium

The supply and demand curves have different slopes/shapes but intersect at approximately the same point. This tests whether results are robust to curve geometry.

### 4.3 Market 3

**Equilibrium**: Price ≈ 106, Quantity = 6 units (low volume)  
**Design Feature**: Small total surplus available

With only 6 equilibrium units, there's limited surplus to extract. This tests efficiency in thin markets.

### 4.4 Market 4

**Equilibrium**: Price ≈ 170, Quantity = high  
**Design Feature**: High equilibrium price level

Tests whether results hold when equilibrium is in the upper portion of the price range.

### 4.5 Market 5

**Equilibrium**: Price ≈ 131, Quantity = 24 units (high volume)  
**Design Feature**: Crossing curves with marginal units clustered near equilibrium

Multiple buyer values and seller costs are positioned just above and below the equilibrium price. This makes it inherently difficult to achieve 100% efficiency because marginal units have very small surplus and can easily be displaced by extramarginal units.

---

## 5. Simulation Protocol

### 5.1 Random Trader Selection

```
FUNCTION run_trading_period(traders, duration):
    order_book = empty OrderBook
    transactions = []

    WHILE time_remaining(duration):
        # Select random trader
        trader = RANDOM_CHOICE(traders)

        # Skip if trader has no units left
        IF trader.units_remaining == 0:
            CONTINUE

        # Generate and process order
        order = trader.generate_order()
        IF order IS NOT NULL:
            result = process_order(order, order_book)
            IF result IS transaction:
                transactions.APPEND(result)
                UPDATE trader inventories

    RETURN transactions
```

### 5.2 Period and Session Structure

```
FUNCTION run_market_session(market_config, trader_type, num_periods=6):
    all_results = []

    FOR period IN 1..num_periods:
        # Reset trader endowments to initial values
        traders = initialize_traders(market_config, trader_type)

        # Run trading period
        period_transactions = run_trading_period(traders, period_duration)

        # Calculate period metrics
        metrics = calculate_metrics(period_transactions, market_config)
        all_results.APPEND(metrics)

    RETURN all_results
```

### 5.3 Replication Count

For ZI traders, the paper ran simulations generating enough data for statistical reliability. A recommended approach:

- **ZI-U**: Run 1000+ sessions per market configuration
- **ZI-C**: Run 1000+ sessions per market configuration
- Human trader data was from single experimental sessions (the original laboratory data)

---

## 6. Metrics and Measurements

### 6.1 Allocative Efficiency

```
FUNCTION calculate_efficiency(transactions, market_config):
    # Calculate actual surplus extracted
    actual_surplus = 0
    FOR t IN transactions:
        buyer_surplus = t.buyer_value - t.price
        seller_surplus = t.price - t.seller_cost
        actual_surplus += buyer_surplus + seller_surplus

    # Calculate maximum possible surplus (at competitive equilibrium)
    max_surplus = calculate_equilibrium_surplus(market_config)

    efficiency = (actual_surplus / max_surplus) * 100
    RETURN efficiency
```

**Equilibrium Surplus Calculation**:
Sort all buyer values descending, all seller costs ascending. Match highest-value buyers with lowest-cost sellers until marginal buyer value < marginal seller cost. Sum (value - cost) for all matched pairs.

### 6.2 Price Deviation from Equilibrium

```
FUNCTION calculate_price_deviation(transactions, equilibrium_price):
    deviations = []
    FOR t IN transactions:
        deviation = t.price - equilibrium_price
        deviations.APPEND(deviation)

    # Root mean squared deviation
    rmsd = SQRT(MEAN(deviation^2 for deviation in deviations))
    RETURN rmsd
```

### 6.3 Profit Dispersion

```
FUNCTION calculate_profit_dispersion(actual_profits, equilibrium_profits):
    # For each trader, compare actual profit to theoretical equilibrium profit
    differences = []
    FOR trader_id IN traders:
        diff = actual_profits[trader_id] - equilibrium_profits[trader_id]
        differences.APPEND(diff^2)

    rmsd = SQRT(MEAN(differences))
    RETURN rmsd
```

### 6.4 Convergence Analysis

For each trading period, regress price deviation on transaction sequence number:

```
deviation_t = α + β * sequence_number_t + ε_t

Expected results for ZI-C:
- β < 0 (negative slope, indicating convergence)
- β significant (p < 0.05)
- R² moderate to high (0.4 - 0.8)
```

---

## 7. Expected Results

### 7.1 Allocative Efficiency by Trader Type

| Trader Type | Expected Efficiency Range | Notes |
|-------------|--------------------------|-------|
| ZI-U | 48% - 90% | Highly variable, depends on market structure |
| ZI-C | 97% - 100% | Consistently high across all markets |
| Human | 90% - 100% | High with occasional shortfalls |

**Key Prediction**: ZI-C efficiency should be statistically indistinguishable from human efficiency.

### 7.2 Price Behavior

| Trader Type | Convergence Pattern | Variance |
|-------------|--------------------| ---------|
| ZI-U | None (random walk) | Very high |
| ZI-C | Within-period convergence | Moderate |
| Human | Rapid convergence, then stability | Low |

**Key Prediction**: ZI-C prices should show significant negative correlation with transaction sequence number within periods.

### 7.3 Efficiency by Market

| Market | ZI-U Expected | ZI-C Expected | Notes |
|--------|---------------|---------------|-------|
| 1 | ~90% | ~100% | Standard design |
| 2 | ~90% | ~99% | Similar to Market 1 |
| 3 | ~77% | ~99% | Low volume market |
| 4 | ~49% | ~98% | High price equilibrium |
| 5 | ~86% | ~97% | Difficult margin structure |

**Key Prediction**: ZI-U efficiency should vary substantially across markets while ZI-C efficiency remains stable.

### 7.4 Profit Distribution

| Trader Type | Profit Dispersion (RMSD from equilibrium) |
|-------------|------------------------------------------|
| ZI-U | High (150-350 range) |
| ZI-C | Moderate (20-60 range) |
| Human | Low (10-30 range) |

**Key Prediction**: Human traders should achieve profit distribution closest to competitive equilibrium predictions.

---

## 8. Validation Criteria

A successful replication should demonstrate:

1. **ZI-C Efficiency**: Mean efficiency ≥ 97% across all five markets
2. **ZI-U vs ZI-C Gap**: ZI-C efficiency exceeds ZI-U by 10+ percentage points on average
3. **ZI-C Convergence**: Significant negative slope in price-deviation regression for ZI-C
4. **ZI-U Non-Convergence**: No significant trend in ZI-U price series
5. **Profit Dispersion Ordering**: ZI-U > ZI-C in profit dispersion (RMSD)
6. **Market Structure Sensitivity**: ZI-U efficiency varies more across markets than ZI-C

---

## 9. Implementation Notes

### 9.1 Random Number Generation

Use a high-quality PRNG with proper seeding. The uniform distribution over integers is critical—any bias will affect results.

```
# Recommended: Use discrete uniform distribution
# NOT: continuous uniform rounded to integer (creates edge bias)

price = random.randint(lower_bound, upper_bound)  # Inclusive bounds
```

### 9.2 Order of Operations

The exact sequence of trader selection and order processing matters:

1. Select trader uniformly at random from all traders with remaining units
2. Generate order according to trader type
3. Check for crossing with existing best quote
4. Execute transaction OR update order book
5. Clear any stale quotes (original paper cancels unaccepted orders)

### 9.3 Edge Cases

- **No feasible price**: If a ZI-C seller's cost equals 200, they can only offer at exactly 200
- **Exhausted traders**: Traders with no remaining units should be skipped, not generate null orders
- **Empty order book**: First orders of a period establish the initial bid/ask

### 9.4 Statistical Analysis

For comparing ZI-C to human efficiency:
- Use t-tests or Mann-Whitney U for efficiency comparisons
- Report confidence intervals, not just point estimates
- Account for the fact that ZI results are simulated (many samples) while human results may be from limited sessions

---

## 10. Extensions and Variations

### 10.1 Testing the Budget Constraint

Run ZI-C with progressively relaxed constraints:
- Allow bids up to value + ε
- Allow asks down to cost - ε
- Measure efficiency degradation as ε increases

### 10.2 Market Size Effects

Vary the number of traders (e.g., 6, 12, 24, 48) while keeping supply/demand curves proportional. Test whether ZI-C efficiency scales.

### 10.3 Information Conditions

Test ZI-C with:
- No market information (baseline, as in paper)
- Observable best bid/ask
- Observable transaction history

ZI-C by definition won't use this information, but it allows comparison with minimally intelligent agents that do.

### 10.4 Alternative Auction Mechanisms

Apply ZI-C traders to:
- Call markets (batch auctions)
- Posted-offer markets
- Sealed-bid auctions

Test whether the efficiency result is specific to continuous double auctions.

---

## Appendix A: Pseudocode for Complete Simulation

```
# Main simulation runner

CONSTANTS:
    PRICE_MIN = 1
    PRICE_MAX = 200
    NUM_TRADERS = 12
    NUM_BUYERS = 6
    NUM_SELLERS = 6
    NUM_PERIODS = 6
    ORDERS_PER_PERIOD = 500  # Sufficient for convergence

CLASS Trader:
    id: int
    role: BUYER | SELLER
    units: List[Unit]  # Each unit has value (buyer) or cost (seller)
    current_unit_index: int = 0

    FUNCTION get_current_unit():
        IF current_unit_index < LENGTH(units):
            RETURN units[current_unit_index]
        RETURN null

    FUNCTION complete_trade():
        current_unit_index += 1

CLASS ZI_U_Trader(Trader):
    FUNCTION generate_order():
        unit = get_current_unit()
        IF unit IS NULL: RETURN null

        price = RANDINT(PRICE_MIN, PRICE_MAX)
        RETURN Order(type=role_to_order_type(role), price=price)

CLASS ZI_C_Trader(Trader):
    FUNCTION generate_order():
        unit = get_current_unit()
        IF unit IS NULL: RETURN null

        IF role == BUYER:
            price = RANDINT(PRICE_MIN, unit.value)
        ELSE:
            price = RANDINT(unit.cost, PRICE_MAX)

        RETURN Order(type=role_to_order_type(role), price=price)

CLASS Market:
    best_bid: Order = null
    best_ask: Order = null

    FUNCTION submit_order(order):
        IF order.type == BID:
            IF best_ask AND order.price >= best_ask.price:
                # Execute trade
                txn = Transaction(
                    price=best_ask.price,
                    buyer=order.trader,
                    seller=best_ask.trader,
                    buyer_value=order.trader.get_current_unit().value,
                    seller_cost=best_ask.trader.get_current_unit().cost
                )
                order.trader.complete_trade()
                best_ask.trader.complete_trade()
                best_ask = null
                RETURN txn
            ELSE:
                best_bid = order
                RETURN null

        ELSE:  # ASK
            IF best_bid AND order.price <= best_bid.price:
                # Execute trade
                txn = Transaction(
                    price=best_bid.price,
                    buyer=best_bid.trader,
                    seller=order.trader,
                    buyer_value=best_bid.trader.get_current_unit().value,
                    seller_cost=order.trader.get_current_unit().cost
                )
                best_bid.trader.complete_trade()
                order.trader.complete_trade()
                best_bid = null
                RETURN txn
            ELSE:
                best_ask = order
                RETURN null

FUNCTION run_period(traders, market):
    transactions = []

    FOR i IN 1..ORDERS_PER_PERIOD:
        # Get traders with units remaining
        active = [t FOR t IN traders IF t.get_current_unit() IS NOT NULL]
        IF LENGTH(active) == 0: BREAK

        trader = RANDOM_CHOICE(active)
        order = trader.generate_order()

        IF order:
            txn = market.submit_order(order)
            IF txn:
                transactions.APPEND(txn)

    RETURN transactions

FUNCTION run_session(market_config, trader_class, num_periods=6):
    period_results = []

    FOR period IN 1..num_periods:
        traders = initialize_traders(market_config, trader_class)
        market = Market()

        transactions = run_period(traders, market)

        efficiency = calculate_efficiency(transactions, market_config)
        price_rmsd = calculate_price_rmsd(transactions, market_config.equilibrium_price)

        period_results.APPEND({
            'period': period,
            'num_transactions': LENGTH(transactions),
            'efficiency': efficiency,
            'price_rmsd': price_rmsd,
            'transactions': transactions
        })

    RETURN period_results

# Run full experiment
FOR market_config IN [MARKET_1, MARKET_2, MARKET_3, MARKET_4, MARKET_5]:
    FOR trader_class IN [ZI_U_Trader, ZI_C_Trader]:
        all_sessions = []
        FOR session IN 1..1000:
            results = run_session(market_config, trader_class)
            all_sessions.APPEND(results)

        REPORT aggregate_statistics(all_sessions)
```

---

## Appendix B: Supply and Demand Schedule Template

```
# Template for defining market configurations

CLASS MarketConfig:
    name: str
    equilibrium_price: float
    equilibrium_quantity: int

    # Buyer values: List of (buyer_id, [unit1_value, unit2_value, ...])
    buyer_values: List[Tuple[int, List[int]]]

    # Seller costs: List of (seller_id, [unit1_cost, unit2_cost, ...])
    seller_costs: List[Tuple[int, List[int]]]

    FUNCTION calculate_equilibrium_surplus():
        # Flatten and sort
        all_values = sorted([v for _, values in buyer_values for v in values], reverse=True)
        all_costs = sorted([c for _, costs in seller_costs for c in costs])

        surplus = 0
        quantity = 0
        FOR v, c IN ZIP(all_values, all_costs):
            IF v >= c:
                surplus += (v - c)
                quantity += 1
            ELSE:
                BREAK

        RETURN surplus, quantity
```

---

## References

Gode, D. K., & Sunder, S. (1993). Allocative efficiency of markets with zero-intelligence traders: Market as a partial substitute for individual rationality. *Journal of Political Economy*, 101(1), 119-137.

Smith, V. L. (1962). An experimental study of competitive market behavior. *Journal of Political Economy*, 70(2), 111-137.

Smith, V. L. (1976). Experimental economics: Induced value theory. *American Economic Review*, 66(2), 274-279.

Becker, G. S. (1962). Irrational behavior and economic theory. *Journal of Political Economy*, 70(1), 1-13.
