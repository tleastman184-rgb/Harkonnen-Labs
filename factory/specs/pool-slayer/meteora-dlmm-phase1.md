# Meteora DLMM Strategy — Pool Slayer Strategy Module
## Harkonnen Spec: `meteora-dlmm-phase1`

---

## Objective

Deploy concentrated liquidity positions on Meteora DLMM SOL/USDC pools
using the same three-layer Belt Buckle approach (blade/middle/anchor) but
with Meteora's dynamic fee advantage. Integrated into Pool Slayer so the
capital allocator can compare Orca vs Meteora performance and shift capital
to whichever earns more net yield.

---

## Why Meteora Over Orca

1. Dynamic fees — fees increase during volatility, earning MORE during the
   exact conditions that cause Belt Buckle churn losses on Orca
2. Idle capital earns lending yield — out-of-range bins auto-lend to Kamino/Solend
3. Bin-based system — zero-slippage within each bin, more precise positioning
4. Higher Fee/TVL — SOL-USDC Bin Step 4 pool shows 0.38% daily Fee/TVL vs
   Orca's lower effective rate

---

## Architecture

### Files

```
pool-slayer/bot/src/
  belt-buckle-v2.ts          ← existing Orca strategy
  meteora-dlmm.ts            ← NEW: Meteora DLMM strategy
  capital-allocator.ts       ← NEW: decides Orca vs Meteora split
  strategies.ts              ← NEW: common strategy interface

pool-slayer/bot/data/
  meteora-positions.json     ← position state
  meteora-fees.json          ← fee collection log
  capital-allocation.json    ← current allocation
```

### Strategy Interface (shared with Belt Buckle)

```typescript
interface Strategy {
  name: string;
  protocol: 'orca' | 'meteora';
  status: 'active' | 'idle' | 'error';
  capitalDeployed: number;
  dailyYieldEstimate: number;
  dailyYieldActual: number;

  deploy(capitalUsd: number): Promise<boolean>;
  withdraw(): Promise<number>;
  getMetrics(): StrategyMetrics;
}
```

---

## Pool Selection

Target pool: SOL-USDC, Bin Step 4, Fee 0.04%
- TVL: $3.5M
- 24h Volume: $31M
- Fee/TVL: 0.38%
- Direct comparison to Orca 0.04% pool

Pool address: lookup at runtime via DLMM SDK
```typescript
import DLMM from '@meteora-ag/dlmm';
const dlmmPool = await DLMM.create(connection, poolAddress);
```

---

## Position Strategy — Three Layers

Same Belt Buckle philosophy, adapted for Meteora bins:

```
Layer     | Capital | Bin Range       | Purpose
----------|---------|-----------------|------------------
Blade     | 20%     | ±4 bins (~0.9%) | Tight, high fee capture
Middle    | 60%     | ±8 bins (~1.8%) | Core earning position  
Anchor    | 20%     | ±16 bins (~3.6%)| Safety net, wide range
```

### Bin Step 4 Math
- Each bin = ~0.04% price increment (1.0004^4 per step)
- 4 bins = ~0.16% range per side
- Blade at ±4 bins ≈ ±0.9% from current price
- Middle at ±8 bins ≈ ±1.8%
- Anchor at ±16 bins ≈ ±3.6%

---

## Core Operations

### Open Position

```typescript
import DLMM from '@meteora-ag/dlmm';
import { StrategyType } from '@meteora-ag/dlmm';

const dlmmPool = await DLMM.create(connection, poolAddress);
const activeBin = await dlmmPool.getActiveBin();
const currentPrice = activeBin.price;

// Create position with bins around current price
const createPositionTx = await dlmmPool.initializePositionAndAddLiquidityByStrategy({
  positionPubKey: newPositionKeypair.publicKey,
  user: wallet.publicKey,
  totalXAmount: solAmount,
  totalYAmount: usdcAmount,
  strategy: {
    maxBinId: activeBin.binId + 8,   // upper range
    minBinId: activeBin.binId - 8,   // lower range
    strategyType: StrategyType.SpotBalanced,  // even distribution
  },
});
```

### Close Position / Collect Fees

```typescript
// Claim fees first
const claimFeeTx = await dlmmPool.claimFee({
  owner: wallet.publicKey,
  position: positionPubKey,
});

// Remove liquidity
const removeLiqTx = await dlmmPool.removeLiquidity({
  position: positionPubKey,
  user: wallet.publicKey,
  binIds: position.positionData.positionBinData.map(b => b.binId),
  bps: new BN(10000),  // 100%
  shouldClaimAndClose: true,
});
```

### Check If In Range

```typescript
const activeBin = await dlmmPool.getActiveBin();
const position = await dlmmPool.getPosition(positionPubKey);
const bins = position.positionData.positionBinData;
const minBin = Math.min(...bins.map(b => b.binId));
const maxBin = Math.max(...bins.map(b => b.binId));
const inRange = activeBin.binId >= minBin && activeBin.binId <= maxBin;
```

---

## Rebalance Logic

Same as Belt Buckle but with two improvements:

1. **Dynamic fee awareness** — check current fee multiplier before rebalancing.
   If volatility fee is high (>2x base), delay rebalance because fees are
   compensating for the range pressure.

2. **Only rebalance the layer that's out of range** — identical to Belt Buckle
   philosophy, never close all three.

```
Every 30 minutes (same as Belt Buckle):
  1. Get active bin from pool
  2. For each layer (blade, middle, anchor):
     a. Check if active bin is within position's bin range
     b. If out of range AND dynamic fee multiplier < 1.5x:
        - Claim fees
        - Remove liquidity  
        - Reopen centered at current active bin
     c. If out of range BUT dynamic fee multiplier >= 1.5x:
        - Log "high volatility fee — holding position"
        - Skip rebalance (fees compensating for being near edge)
  3. Log all activity with [MET] prefix
```

---

## Fee Collection

Unlike Orca where fees are collected on close, Meteora requires manual claiming:

```
Every 4 hours:
  1. Call claimFee for each position
  2. Log amounts to meteora-fees.json
  3. Update dashboard with cumulative fees
```

---

## Data Logging

### meteora-positions.json
```json
[
  {
    "label": "blade",
    "positionPubKey": "...",
    "poolAddress": "...",
    "minBinId": 1234,
    "maxBinId": 1242,
    "sizeUsd": 18.50,
    "openedAt": "2026-04-17T...",
    "feesCollected": { "sol": 0.001, "usdc": 0.05 }
  }
]
```

### meteora-fees.json
```json
[
  {
    "timestamp": "2026-04-17T12:00:00Z",
    "layer": "middle",
    "solFees": 0.000543,
    "usdcFees": 0.0412,
    "totalUsd": 0.09,
    "dynamicFeeMultiplier": 1.2
  }
]
```

---

## Dashboard Integration

Add Meteora section to Belt Buckle Command Center:

### New API Endpoint: `/api/meteora`

```json
{
  "status": "active",
  "positions": 3,
  "totalDeployed": 80.00,
  "totalFees": 1.45,
  "dynamicFeeMultiplier": 1.3,
  "currentBinId": 1238,
  "blade": { "inRange": true, "sizeUsd": 16.00, "fees": 0.32 },
  "middle": { "inRange": true, "sizeUsd": 48.00, "fees": 0.85 },
  "anchor": { "inRange": true, "sizeUsd": 16.00, "fees": 0.28 }
}
```

### Dashboard Panel

```
┌─────────────────────────────────────────┐
│  METEORA DLMM         ● ACTIVE          │
│  SOL/USDC Bin Step 4                    │
│                                         │
│  Deployed: $80.00   Fees: $1.45         │
│  Dynamic Fee: 1.3x base                │
│  ⚔ Blade  IR  🛡 Middle  IR  ⚓ Anchor IR │
└─────────────────────────────────────────┘
```

---

## Capital Allocator

Compares Orca Belt Buckle vs Meteora DLMM:

```
Every 4 hours:
  orca_yield = belt_buckle_trailing_7day_net_pct
  meteora_yield = meteora_trailing_7day_net_pct

  if meteora_yield > orca_yield * 1.3:
    # Meteora significantly better — shift capital
    meteora_allocation = 70%
    orca_allocation = 30%

  elif orca_yield > meteora_yield * 1.3:
    # Orca significantly better — shift capital  
    orca_allocation = 70%
    meteora_allocation = 30%

  else:
    # Similar performance — split evenly
    orca_allocation = 50%
    meteora_allocation = 50%

  # Always keep 10% cash reserve
  # Log decision with reasoning to capital-allocation.json
```

---

## Thufir Integration

Observer tracks both protocols with parallel metrics:

```yaml
meteora_metrics:
  - daily_fees_collected_usd
  - dynamic_fee_multiplier_avg
  - rebalance_count
  - time_in_range_pct
  - idle_capital_lending_yield
  - net_yield_vs_orca (comparative)
```

Observer can now diagnose:
- ORCA_BETTER: Orca outperforming, shift capital
- METEORA_BETTER: Meteora outperforming, shift capital
- DYNAMIC_FEE_ADVANTAGE: Meteora earning more during volatility spikes
- LENDING_YIELD_BONUS: Idle capital earning meaningful lending yield

---

## Build Order

### Phase 1: Single Position Test (1 session)

1. Connect to Meteora SOL-USDC Bin Step 4 pool via SDK
2. Open one middle-layer position (±8 bins)
3. Verify position appears on Meteora UI
4. Claim fees after 4 hours
5. Close position cleanly
6. Log all operations

### Phase 2: Three-Layer Deploy (1 session)

1. Deploy blade/middle/anchor with proper capital split
2. Add 30-minute check cycle (same as Belt Buckle)
3. Add rebalance logic with dynamic fee awareness
4. Add fee claiming every 4 hours
5. Write to meteora-positions.json and meteora-fees.json

### Phase 3: Dashboard + Comparison (1 session)

1. Add /api/meteora endpoint to dashboard server
2. Add Meteora panel to HTML dashboard
3. Run both strategies for 7 days
4. Compare net yields

### Phase 4: Capital Allocator (after 7 days of data)

1. Enable capital-allocator.ts
2. Auto-shift capital based on trailing performance
3. Factory optimizes allocation thresholds

---

## Non-Negotiable Constraints

1. Same wallet, same SOL reserve rules as Belt Buckle
2. Emergency exit converts everything to USDC (same trigger)
3. Never deploy more than 60% of total capital to Meteora
4. All position changes logged with timestamp and reason
5. 30-minute check interval (matching Belt Buckle)
6. Dynamic fee multiplier logged on every check
7. Telegram notification on every open, close, and fee claim

---

## Success Criteria

After 14 days of parallel operation:

- Meteora net yield measurably compared to Orca net yield
- Dynamic fee advantage quantified (extra fees during volatility)
- Lending yield on idle capital quantified
- Capital allocator has enough data to make informed decisions
- Combined portfolio outperforms single-strategy portfolio
- Zero positions stuck as ghosts (learned from Belt Buckle bug)

---

## Key Advantage Over Belt Buckle

The dynamic fee skip rule is the critical innovation:

When SOL is volatile and positions are near the edge, Belt Buckle
rebalances and pays slippage. Meteora charges HIGHER fees during
this exact moment. Instead of rebalancing (paying slippage to
reposition), Meteora DLMM earns MORE from the volatility that
drives trades through the pool.

The factory hypothesis: dynamic fees + skip-rebalance-during-volatility
will produce higher net yield than fixed fees + aggressive rebalancing.

14 days of parallel data will prove or disprove this.
