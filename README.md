# CGM Insights API

Syncs FreeStyle Libre (LibreLink Up) glucose data into Postgres and exposes it over a JWT-protected API. Dexcom is not implemented yet.

## Prerequisites

- Rust (stable)
- PostgreSQL

## Setup

1. Create a database and copy the env file:

```bash
cp .env.example .env.development
```

1. Set at least:


| Variable             | Required | Notes                                                |
| -------------------- | -------- | ---------------------------------------------------- |
| `DATABASE_URL`       | yes      | Postgres connection string                           |
| `JWT_SECRET`         | yes      | Any long random string                               |
| `APP_ENV`            | no       | Defaults to `development` (loads `.env.development`) |
| `PORT`               | no       | Defaults to `3000`                                   |
| `SYNC_INTERVAL_SECS` | no       | Worker sync interval; default `36000` (10 hours)     |
| `RUST_LOG`           | no       | Defaults to `info`                                   |


Loads `.env.{APP_ENV}` first, then falls back to `.env`.

1. Run the API:

```bash
cargo run
```

- Health: `http://127.0.0.1:3000/health`
- Swagger: `http://127.0.0.1:3000/swagger-ui/`

`cargo run` starts the HTTP API only. Background sync needs a worker process (see below).

## CGM credentials (put them in the database)

LibreLink Up email/password are stored per user in `cgm_credentials`.

**Option A — signup (creates user + first credential):**

```http
POST /api/auth/signup
Content-Type: application/json

{
  "username": "alice",
  "password": "your-app-password",
  "cgmUsername": "libreview@email.com",
  "cgmPassword": "libreview-password",
  "cgmType": "FreeStyle",
  "cgmRegion": "eu"
}
```

`cgmType` is `FreeStyle` (or `Dexcom`, not wired yet); matching is case-insensitive and unknown types are rejected with `400`. `cgmRegion` examples: `eu`, `us` (defaults to `eu` for FreeStyle).

**Option B — after login**, manage credentials under `/api/cgm` (Bearer token required):


| Method   | Path                   | Purpose          |
| -------- | ---------------------- | ---------------- |
| `GET`    | `/api/cgm`             | List credentials |
| `POST`   | `/api/cgm`             | Add credential   |
| `PATCH`  | `/api/cgm/{id}`        | Update           |
| `POST`   | `/api/cgm/{id}/active` | Set active       |
| `DELETE` | `/api/cgm/{id}`        | Delete           |


Only **active** credentials are synced. Readings are stored under that user.

**Pull data now:**

```http
POST /api/sync
Authorization: Bearer <token>
```

**Readings:**

```http
GET /api/glucose
Authorization: Bearer <token>
```



## Background sync (worker)

The scheduler loads all active credentials, syncs once on start, then every `SYNC_INTERVAL_SECS`.

```bash
FLY_PROCESS_GROUP=worker cargo run
```

On Fly, the `worker` process group runs this automatically. Pushing to `main`/`master` deploys via GitHub Actions. Manual deploy:

```bash
./deploy.sh
# or: fly deploy --build-arg GIT_REV_SHORT=$(git rev-parse --short=7 HEAD)
```



## Auth & other endpoints


| Method | Path               | Auth | Notes                         |
| ------ | ------------------ | ---- | ----------------------------- |
| `POST` | `/api/auth/signup` | no   | Creates user + CGM credential |
| `POST` | `/api/auth/login`  | no   | Returns JWT                   |
| `GET`  | `/api/glucose`     | yes  | List readings                 |
| `POST` | `/api/sync`        | yes  | Sync active credential now    |
| `GET`  | `/api/cgm`         | yes  | Credential CRUD (see above)   |
| `GET`  | `/health`          | no   | Health check                  |




## Errors

All errors return `{ "error": "<message>" }`.

| Status | Meaning |
| ------ | ------- |
| `400`  | Invalid input |
| `401`  | Missing/invalid token, or wrong username/password |
| `404`  | Resource not found (or owned by another user) |
| `409`  | Username already exists |
| `422`  | CGM platform rejected the stored credentials (wrong password, region, terms to accept in the vendor app) |
| `429`  | CGM account temporarily locked by the vendor, or rate limit hit |
| `502`  | CGM platform unreachable or returned something unexpected |

## Adding a CGM platform

Everything platform-specific sits behind one `match` in `src/services/sync_service.rs`:

1. Add a variant to `CgmType` (`src/models/cgm_credential.rs`) with its canonical name.
2. Add a client in `src/services/` returning `Vec<NewGlucoseReading>` in **mg/dL**, with trends mapped onto the existing strings (`singleDown` … `notComputable`).
3. Map its errors onto `CgmCredentialsRejected` / `CgmAccountLocked` / `ApiError` so the worker alerts only on real failures.
4. Add the arm in `fetch_readings`; the compiler flags it if missed.

`prepare_readings` then drops implausible values and assigns the user, and `insert_many` dedupes and upserts in chunks, for every platform.

## Tests

```bash
cargo test
```

Unit tests cover validation, credential normalization, error mapping, reading mapping and dedupe; `tests/http.rs` exercises routing, auth, CORS and validation without a database. CI runs them before every deploy.

## Sentry (optional)

Set `SENTRY_DSN` to enable error tracking. Omit it and Sentry stays off.

Optional: `SENTRY_RELEASE`, `SENTRY_TRACES_SAMPLE_RATE` (default `1.0`).