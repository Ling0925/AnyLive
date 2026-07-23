# OpenAPI consumer contract

**Source of truth for clients (Admin / H5 / Flutter generated types):**  
`contracts/openapi/openapi.yaml`

## Why not utoipa `ApiDoc` alone?

`backend/crates/api` still exposes a utoipa `ApiDoc` for in-process OpenAPI tests and
optional swagger wiring. That `paths()` list is **intentionally incomplete** relative
to the live router (export, legal, pay/*, feed, admin gifts/reports, compliance, …).

For P1:

- **Clients and CI** consume **only** `openapi.yaml`.
- `scripts/validate-contracts.py` enforces required paths on the YAML.
- `scripts/gen-openapi-ts.sh` generates Admin/H5 TypeScript from the YAML.
- utoipa `ApiDoc` is **not** the wire contract; do not treat swagger export as
  authoritative until a dedicated sync job (or full `paths()` expansion) lands.

FEATURE_PK / FEATURE_COHOST remain **default OFF**. Their routes may appear in
YAML for freeze/scaffolding; runtime returns feature-disabled when flags are off.

## Change process

1. Edit `openapi.yaml` (+ error codes if needed).
2. Run `python scripts/validate-contracts.py`.
3. Run `./scripts/gen-openapi-ts.sh` and commit generated types.
4. Implement or adjust handlers under `backend/crates/api` to match.
