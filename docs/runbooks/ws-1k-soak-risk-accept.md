# Risk accept — P1 WS gate without stage 15 min soak

> **Status: unsigned draft** (plan 06 §8.3 exit quality / decision D3)  
> **Does not** close a full stage 15-minute soak. Prefer running soak when stage exists.

## Context

| Item | Value |
|---|---|
| Local baseline | `reports/ws-1k-baseline-20260722T121825Z.md` |
| Load tool | `scripts/loadtest/ws-centrifugo-load.py` |
| Result (local) | 1000/1000 connect, est. loss 0%, **hold ~180s** (not 15 min) |
| Status note | `reports/ws-1k-soak-status-20260722.md` (gate still OPEN) |
| Related | plan 06 D3 · [p1-status](../product/p1-status.md) · capacity notes in product 04 |

## Gate vs evidence

| Gate item | Target | Local baseline | Stage 15 min |
|---|---|---|---|
| Concurrent WS | 1000 | Met | Not run |
| Message loss | <0.1% | Met @ ~3 min | Unknown |
| Duration | **15 min** | **Not met** (180s) | Not run |
| Chat E2E P95 | ≤500ms | Not measured | Not measured |
| API P95 under load | ≤300ms | Not measured | Not measured |
| Topology | stage | localhost | Missing |

## What is accepted if signed

- P1 may proceed to **internal dogfood** treating the **local 1000 × ~3 min** Centrifugo baseline as **interim** WS evidence.
- Exit tables may note “WS: local baseline + risk-accept” instead of “stage soak green”.
- Does **not** authorize production / GA capacity claims.

## What is **not** accepted

- Claiming “15 min 1k soak passed”.
- Claiming stage topology, multi-AZ, or production Centrifugo limits were proven.
- Skipping a scheduled stage soak when stage is available without re-signing.
- Using this form to mark device matrix, OBS week, or defect council complete.

## Residual risks

1. **Hold duration:** connection stability past 3 minutes unknown (idle timeout, memory leak, reconnect storms).
2. **Topology gap:** localhost compose ≠ stage LB / TLS / multi-instance Centrifugo.
3. **Cross-service pressure:** chat publish + gift fan-out P95 under 1k not measured.
4. **Regression blindness:** future merges may break WS without a long soak CI gate.

## Mitigations while accepted

- Keep `reports/ws-1k-soak-status-20260722.md` checkboxes honest (15 min OPEN).
- Before any public or paid traffic claim: run **15 min × 1000** on stage and archive a new report.
- Optionally measure chat E2E P95 and API P95 under load when claiming capacity exit.
- Record Centrifugo version + API commit in the next measured report.

## Sign-off (leave blank until decided)

| Role | Name | Date (UTC) | Signature |
|---|---|---|---|
| Tech lead |  |  |  |
| SRE / ops (if any) |  |  |  |

**Checkbox (signer only):**

- [ ] I accept local 180s / 1000-client baseline as **interim** P1 WS evidence only
- [ ] Residual risks above acknowledged (no 15 min hold, no stage topology, no load P95)
- [ ] Follow-up: schedule stage 15 min soak before GA / production capacity claim
- [ ] I will not rewrite soak status reports as “passed” based on this document alone

**Preferred close path (if stage appears):**

```bash
# Example — adjust API_BASE / CENTRIFUGO_WS / HOLD_SECS for stage
HOLD_SECS=900 CONCURRENCY=1000 ./scripts/loadtest/ws-centrifugo-load.py
# Archive under reports/ws-1k-baseline-<UTC>.md and update soak-status
```

Notes: _______________
