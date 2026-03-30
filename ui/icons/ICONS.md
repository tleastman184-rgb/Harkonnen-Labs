# Harkonnen Labs — Icon Pack File Map

Place cropped images from the nano banana sheet at the paths below.
Each agent directory holds four variants. The Pack Board uses `icon.png` by default
and falls back to a generated placeholder if the file is absent.

---

## File Convention

```
ui/icons/agents/<name>/
  icon.png          # Primary — transparent background, full illustration
  card-dark.png     # Dark card variant — dog on dark slate background + nameplate
  card-light.png    # Light card variant — dog on off-white card + nameplate
  silhouette.png    # Inactive — silhouette only, used for idle/disabled status chips
```

---

## Crop Map (from the sheet)

### Row 1 — Transparent background heroes

| Sheet position | Agent   | File                              | Notes |
|---------------|---------|-----------------------------------|-------|
| Row 1, Col 1  | Scout   | `scout/icon.png`                  | Sitting, rolled spec tube satchel, amber nameplate |
| Row 1, Col 2  | Mason   | `mason/icon.png`                  | Hard hat, blueprint, orange nameplate |
| Row 1, Col 3  | Piper   | `piper/icon.png`                  | Utility harness, grey-green nameplate |
| Row 1, Col 4  | Coobie  | `coobie/icon.png`                 | Standing, burgundy collar/medal — use if preferred over archive-box pose |
| Row 1, Col 5  | Keeper  | `keeper/icon-nodes.png`           | Shows geometric nodes — possible Ash mix-up from generator; keep as alt |

### Row 2 — Full illustrations

| Sheet position | Agent   | File                              | Notes |
|---------------|---------|-----------------------------------|-------|
| Row 2, Col 1  | Bramble | `bramble/icon.png`                | Goggles up, checklist clipboard, yellow nameplate |
| Row 2, Col 2  | Sable   | `sable/icon.png`                  | Sentinel pose, locked clipboard, slate nameplate |
| Row 2, Col 3  | Ash     | `ash/icon.png`                    | Network diagram lines, teal nameplate |
| Row 2, Col 4  | Coobie  | `coobie/icon.png` *(preferred)*   | Archive box + field notebook — canonical Coobie pose |
| Row 2, Col 5  | Keeper  | `keeper/icon.png`                 | Standing at gate/door, brass badge — canonical Keeper pose |

### Row 3 — Standalone (no card background)

| Sheet position | Agent   | File                              | Notes |
|---------------|---------|-----------------------------------|-------|
| Row 3, Col 1  | Flint   | `flint/icon.png`                  | Carrying canister/roll, tan nameplate |
| Row 3, Col 2  | Coobie  | *(same as Row 2 Col 4)*           | Second render — use whichever is cleaner |

### Piper variant demo (bottom right)

| Variant               | File                      |
|-----------------------|---------------------------|
| Primary (transparent) | `piper/icon.png`          |
| Dark card             | `piper/card-dark.png`     |
| Light card            | `piper/card-light.png`    |
| Silhouette-only       | `piper/silhouette.png`    |

---

## Missing from this sheet (request from nano banana)

The following agents need icons generated:
- Ash: only the network-diagram version was generated — a cleaner standalone pose would help
- Flint: only one pose generated
- Keeper Row 1 Col 5 may be a mislabeled Ash — clarify with nano banana

---

## Accent Colors (for card border / nameplate)

| Agent   | Hex       | Description              |
|---------|-----------|--------------------------|
| Scout   | `#c4922a` | Amber / document gold    |
| Mason   | `#c4662a` | Construction orange      |
| Piper   | `#5a7a5a` | Industrial grey-green    |
| Bramble | `#a89a2a` | Inspection yellow        |
| Sable   | `#3a4a5a` | Deep graphite / slate    |
| Ash     | `#2a7a7a` | Cool teal                |
| Flint   | `#8a6a3a` | Warm tan / kraft         |
| Coobie  | `#7a2a3a` | Library burgundy         |
| Keeper  | `#8a7a3a` | Deep brass               |
