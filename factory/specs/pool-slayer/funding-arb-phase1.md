# Funding Rate Arbitrage — Pool Slayer Strategy Module
## Harkonnen Spec: `funding-arb-phase1`

---

## Objective

Deploy a delta-neutral funding rate arbitrage strategy on Solana that collects
perpetual futures funding payments while maintaining zero directional exposure.
Integrated into Pool Slayer so the factory (Thufir) can allocate capital between
Belt Buckle (concentrated LP) and Funding Arb based on which strategy has the
higher expected return at any given time.

---

## Strategy Overview

```
When funding rate is high (longs paying shorts):
  1. Hold spot SOL in wallet
  2. Open equal-size SHORT on Drift/Jupiter Perps
  3. Collect funding payments every 8 hours
  4. Net price exposure: zero (spot gains offset perp losses and vice versa)

When funding rate is low or negative:
  1. Close perp position
  2. Return capital to idle pool for Belt Buckle or staking
```

---

## Architecture

### Position in Pool Slayer

```
pool-slayer/
  bot/
    src/
      belt-buckle-v2.ts       ← existing LP strategy
      funding-arb.ts          ← NEW: funding arb strategy
      capital-allocator.ts    ← NEW: decides capital split
      strategies.ts           ← NEW: strategy interface
    data/
      funding-arb-state.json  ← position state
      funding-rates.json      ← historical rate log
      capital-allocation.json ← current allocation decision
```

### Strategy Interface

Both Belt Buckle and Funding Arb implement the same interface so the factory
can treat them uniformly:

```typescript
interface Strategy {
  name: string;
  status: 'active' | 'idle' | 'error';
  capitalDeployed: number;       // USD currently in use
  capitalNeeded: number;         // USD requested for optimal operation
  dailyYieldEstimate: number;    // expected daily return %
  dailyYieldActual: number;      // trailing 7-day actual return %
  riskScore: number;             // 0-100, higher = riskier
  
  deploy(capitalUsd: number): Promise<boolean>;
  withdraw(): Promise<number>;   // returns USD amount freed
  getMetrics(): StrategyMetrics;
}
```

---

## Funding Arb Logic

### Entry Conditions (ALL must be true)

1. Funding rate > 0.03% per 8hrs (= 0.09%/day = 32.85% APY)
2. Rate has been positive for >= 3 consecutive periods (24hrs)
3. Available capital >= $100 (minimum viable position)
4. No active funding arb position already open
5. Spread between spot and perp mark price < 0.5%

### Exit Conditions (ANY triggers exit)

1. Funding rate turns negative for 2 consecutive periods
2. Funding rate drops below 0.01% per 8hrs for 3 consecutive periods
3. Unrealized PnL on perp exceeds -5% of position size (safety stop)
4. SOL drops > 15% in 1 hour (emergency exit — same as Belt Buckle)
5. Manual trigger via dashboard or API

### Position Sizing

```
total_capital = wallet SOL value + wallet USDC
allocation = capital_allocator decision (0-100% of total)
position_size = allocation / 2    # half spot, half margin

spot_sol = position_size / sol_price
perp_short = spot_sol              # equal and opposite

margin_requirement = position_size * 0.10  # 10x leverage = 10% margin
usdc_for_margin = position_size * 0.50     # keep 50% as margin for safety
                                            # (effective 2x, very conservative)
```

### Funding Collection

```
Every 8 hours (Drift schedule: 00:00, 08:00, 16:00 UTC):
  1. Read current funding rate from Drift API
  2. Calculate payment: rate * position_notional
  3. Log to funding-rates.json
  4. If accumulated funding > $1: optionally harvest to reserve wallet
```

---

## Capital Allocator

The allocator runs once per hour and decides the optimal split:

```
INPUT:
  - Belt Buckle trailing 7-day net yield %
  - Funding Arb current rate (annualized)
  - SOL 24hr volatility
  - Total available capital

LOGIC:
  if funding_rate_annual > belt_buckle_yield * 2:
    # Funding arb is significantly better — shift capital
    funding_arb_allocation = 60%
    belt_buckle_allocation = 40%
  
  elif funding_rate_annual > belt_buckle_yield:
    # Funding arb is slightly better — split evenly
    funding_arb_allocation = 40%
    belt_buckle_allocation = 60%
  
  elif funding_rate_annual < 0:
    # Negative funding — all to Belt Buckle
    funding_arb_allocation = 0%
    belt_buckle_allocation = 100%
  
  else:
    # Belt Buckle is better — default allocation
    funding_arb_allocation = 20%
    belt_buckle_allocation = 80%

  # Never allocate more than 60% to funding arb (perp risk)
  # Always keep 10% as cash reserve

OUTPUT:
  - Write to capital-allocation.json
  - If reallocation needed, trigger withdraw from lower-yield strategy
  - Deploy freed capital to higher-yield strategy
```

---

## Data Requirements

### Drift Protocol Integration

```
Endpoints needed:
  GET funding rate:     https://drift-historical-data-v2.s3.eu-west-1.amazonaws.com/...
  OR Drift SDK:         @drift-labs/sdk
  
  Open perp position:   drift.openPosition(market, size, direction, leverage)
  Close perp position:  drift.closePosition(market)
  Get position:         drift.getUser().getPerpPosition(marketIndex)
  Get funding payments: drift.getUser().getFundingPayments()

SOL-PERP market index: 0 (on Drift)
```

### Alternative: Jupiter Perps

```
  Jupiter Perps SDK:    @jup-ag/perps-sdk
  Simpler API but potentially lower liquidity
  Evaluate both during build — use whichever has better rates
```

### Price Feeds

```
  Spot SOL price:       Jupiter price API (already integrated)
  Perp mark price:      Drift oracle / Jupiter oracle
  Funding rate:         Drift API (historical + current)
```

---

## Risk Management

### Hard Rules (non-negotiable)

1. Maximum perp leverage: 2x (conservative — most arb bots use 5-10x)
2. Maximum capital in funding arb: 60% of total portfolio
3. Stop loss on perp: -5% of position notional
4. Emergency exit: SOL -15% in 1 hour → close everything → convert to USDC
5. Minimum cash reserve: 10% of portfolio always undeployed
6. Never increase position into falling funding rates

### Monitoring

```
Every 15 minutes:
  - Check perp position health (margin ratio)
  - Check spot/perp price divergence
  - Log funding rate to history
  - Update dashboard metrics

Alert thresholds:
  - Margin ratio < 30%: WARNING
  - Margin ratio < 15%: EMERGENCY EXIT
  - Funding rate negative: PREPARE TO EXIT
  - Spot/perp divergence > 1%: WARNING (oracle issue)
```

---

## Dashboard Integration

Add to Belt Buckle Command Center dashboard:

### New API Endpoint: `/api/funding-arb`

```json
{
  "status": "active",
  "spotSol": 0.65,
  "perpShortSol": 0.65,
  "marginUsdc": 29.50,
  "currentFundingRate": 0.0012,
  "fundingRateAnnualized": 52.56,
  "totalFundingCollected": 1.23,
  "positionOpenedAt": "2026-04-17T08:00:00Z",
  "unrealizedPnl": 0.15,
  "marginRatio": 0.45,
  "nextFundingIn": "2h 14m"
}
```

### New Dashboard Section

```
┌─────────────────────────────────────────┐
│  FUNDING ARB          ● ACTIVE          │
│                                         │
│  Rate: 0.12%/8hr  →  43.8% APY         │
│  Collected: $1.23    Since: Apr 17      │
│  Margin Health: ████████░░ 45%          │
│  Next Funding: 2h 14m                   │
│                                         │
│  [Close Position]  [Force Harvest]      │
└─────────────────────────────────────────┘

┌─────────────────────────────────────────┐
│  CAPITAL ALLOCATOR                      │
│                                         │
│  Belt Buckle: 60% ($144)  → 0.15%/day  │
│  Funding Arb: 30% ($72)   → 0.45%/day  │
│  Cash Reserve: 10% ($24)               │
│  ████████████████████░░░░░ ░░░░        │
│  [Rebalance Now]                        │
└─────────────────────────────────────────┘
```

---

## Thufir Integration

### Observer Additions

The Observer should track these metrics for funding arb:

```yaml
funding_arb_metrics:
  - daily_funding_collected_usd
  - funding_rate_trend (rising/falling/stable)
  - time_in_position_hours
  - margin_utilization_pct
  - missed_funding_periods (times rate was high but no position open)
  - capital_efficiency (funding_collected / capital_deployed)
```

### Failure Modes Observer Should Detect

1. MISSED_OPPORTUNITY: Funding rate was > threshold but no position open
2. NEGATIVE_CARRY: Position open during negative funding (paying instead of receiving)
3. MARGIN_SQUEEZE: Margin ratio dropped below 25% requiring emergency action
4. STALE_POSITION: Position open > 7 days with cumulative funding < gas costs
5. ALLOCATION_DRAG: Capital sitting idle when either strategy has positive expected return

---

## Build Order

### Phase 1: Data Collection (no real trades)

1. Fetch Drift funding rates every 15 minutes
2. Log to funding-rates.json with timestamp
3. Add funding rate display to dashboard
4. Run for 7 days to build rate history baseline
5. Backtest: "if we had entered at X rate, what would we have earned?"

### Phase 2: Paper Trading

1. Simulate spot + perp positions based on Phase 1 thresholds
2. Track simulated funding payments
3. Compare simulated returns vs Belt Buckle actual returns
4. Validate entry/exit logic against real rate movements

### Phase 3: Live with Minimum Capital

1. Deploy with $50 ($25 spot + $25 margin)
2. Run for 14 days
3. Measure actual funding collected vs simulated
4. Confirm margin management works correctly

### Phase 4: Capital Allocator Live

1. Enable capital-allocator.ts
2. Allow automatic capital movement between strategies
3. Factory monitors and optimizes allocation thresholds
4. Target: 30 days of data before Thufir adjusts parameters

---

## Non-Negotiable Constraints

1. Never use leverage above 2x on perps
2. Never deploy more than 60% of total capital to funding arb
3. Always maintain ability to exit all positions within 60 seconds
4. Emergency exit converts everything to USDC (same as Belt Buckle)
5. Never chase negative funding — if rate is negative, stay out
6. All position changes logged with timestamp, size, rate, and reason
7. Telegram notification on every entry, exit, and funding collection

---

## Success Criteria

After 30 days of live operation:

- Funding arb net positive (after all costs)
- Combined portfolio (Belt Buckle + Funding Arb) outperforms Belt Buckle alone
- Capital allocator correctly shifts capital toward higher-yield strategy
- Zero liquidations or emergency exits triggered by margin issues
- Factory has enough data to begin autonomous optimization
