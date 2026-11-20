# LibreLink Up API (unofficial notes)

The follower API behind Abbott's **LibreLink Up** app (FreeStyle Libre 1/2/3). Not documented by Abbott; it changes without notice.
We use it through the [`libre_link_up_api_client`](https://github.com/itsXrgon/libre-link-up-api-client) crate (MIT, v0.3.3). Everything here is read from that crate's source, unless marked otherwise.

How it works: the patient's phone uploads to LibreView; a **follower** account (invited from the patient's LibreLink app, with LibreLink Up installed once to accept terms) can read the patient's data. Our stored credential is that follower account.

## Hosts

Login starts at the global host, which redirects to the account's region.

| Region | Base URL |
| ------ | -------- |
| global (default) | `https://api.libreview.io` |
| `ae` | `https://api-ae.libreview.io` |
| `ap` | `https://api-ap.libreview.io` |
| `au` | `https://api-au.libreview.io` |
| `ca` | `https://api-ca.libreview.io` |
| `de` | `https://api-de.libreview.io` |
| `eu` | `https://api-eu.libreview.io` |
| `eu2` | `https://api-eu2.libreview.io` |
| `fr` | `https://api-fr.libreview.io` |
| `jp` | `https://api-jp.libreview.io` |
| `us` | `https://api-us.libreview.io` |
| `la` | `https://api-la.libreview.io` |
| `ru` | `https://api.libreview.ru` |
| `cn` | `https://api-cn.myfreestyle.cn` |

Our `cgm_credentials.region` maps onto these (we default to `eu`). Unknown strings fall back to global.

## Headers (every request)

```
product: llu.ios
version: 4.16.0
content-type: application/json;charset=UTF-8
accept: application/json
accept-encoding: gzip
cache-control: no-cache
connection: Keep-Alive
accept-language: en-US
user-agent: Mozilla/5.0 (iPhone; CPU OS 17_4.1 like Mac OS X) ...
```

`product` and `version` matter: the server rejects app versions it considers too old (`/llu/config/country` reports the minimum).

Authenticated requests add:

```
authorization: Bearer <authTicket.token>
account-id: <lowercase hex SHA-256 of data.user.id from login>
```

## Login

```
POST /llu/auth/login
{"email": "<follower email>", "password": "<password>"}
```

The response is `{status, data}`; `data` takes one of these shapes:

| Case | How to recognize | Meaning |
| ---- | ---------------- | ------- |
| Success | `status: 0`, `data.authTicket{token, expires, duration}`, `data.user{id, ...}` | Store token + user id |
| Redirect | `data: {redirect: true, region: "eu"}` | Retry login on that region's host |
| Bad credentials | `status: 2` | Wrong email/password |
| Action required | `status: 4`, `data.step{type, componentName}` | Terms/privacy to accept, or MFA. User must open the LibreLink Up app. `componentName` is e.g. `tou`, `pp` |
| Locked | `data: {code, message, data: {failures, interval, lockout}}` | Too many failures; wait `lockout` seconds |

Lockouts happen after a handful of failed logins, so repeated logins with a wrong password lock the account for everyone using it.

## Endpoints

All authenticated `GET`s return `{status, data, ticket}`. `ticket` is a refreshed `authTicket`.

| Path | Returns |
| ---- | ------- |
| `/llu/connections` | `data[]`: patients this follower follows |
| `/llu/connections/{patientId}/graph` | Latest reading plus recent history (the one we use) |
| `/llu/connections/{patientId}/logbook` | Logged events and alarms |
| `/llu/notifications/settings/{connectionId}` | Alarm settings |
| `/user`, `/account` | Follower account details |
| `/llu/config/country?country=XX&version=4.16.0` | Unauthenticated: minimum app version, regional host map |

### Connection (`/llu/connections` item, also `graph.data.connection`)

| Field | Meaning |
| ----- | ------- |
| `patientId` | Use in the graph/logbook paths |
| `firstName`, `lastName` | Patient name (the crate can pick a connection by name; we take the first) |
| `targetLow`, `targetHigh` | Patient's target range, mg/dL |
| `uom` | Display unit |
| `sensor{deviceId, sn, a, ...}` | Current sensor; `a` is activation (epoch seconds) |
| `glucoseMeasurement` | Latest reading (shape below) |
| `patientDevice{ll, hl, alarms, ...}` | Reader/phone alarm limits |

### Graph (`/llu/connections/{patientId}/graph`)

```
data.connection        latest reading in connection.glucoseMeasurement
data.activeSensors[]   sensor + device info
data.graphData[]       history readings
```

History coverage (community knowledge, not in the crate): about the last 12 hours, one point every 15 minutes. The "current" reading is usually also in `graphData`, which is why we dedupe by timestamp before inserting.

### Reading (`glucoseMeasurement`, `graphData[]`)

```json
{
  "FactoryTimestamp": "11/20/2026 9:41:00 AM",
  "Timestamp": "11/20/2026 10:41:00 AM",
  "type": 1,
  "ValueInMgPerDl": 112,
  "Value": 6.2,
  "GlucoseUnits": 1,
  "TrendArrow": 3,
  "MeasurementColor": 1,
  "isHigh": false,
  "isLow": false
}
```

| Field | Meaning |
| ----- | ------- |
| `FactoryTimestamp` | **UTC**, format `%m/%d/%Y %I:%M:%S %p`. Use this one |
| `Timestamp` | Same moment in the patient's local time, no offset given |
| `ValueInMgPerDl` | Glucose in mg/dL, always |
| `Value` | Glucose in the **display unit** (`GlucoseUnits`: `0` mg/dL, `1` mmol/L) |
| `TrendArrow` | `1` SingleDown, `2` FortyFiveDown, `3` Flat, `4` FortyFiveUp, `5` SingleUp; `0`/`6` NotComputable. Missing on some history items |
| `isHigh`, `isLow` | Outside the patient's target range |
| `MeasurementColor` | UI colour band (in range / high / low) |

## How the crate maps it, and what to watch

`read()` returns `current` (from `connection.glucoseMeasurement`) and `history` (from `graphData`), each `{value, is_high, is_low, trend, timestamp}`.

- **Unit bug:** `value` comes from `Value`, not `ValueInMgPerDl`. For a follower account set to mmol/L we would store mmol/L numbers as mg/dL. Fix in the crate: read `ValueInMgPerDl`.
- **Timestamp fallback:** if `FactoryTimestamp` doesn't parse, the crate silently uses `Utc::now()`. That stores the reading at the wrong time rather than failing.
- **Missing trend:** missing or out-of-range `TrendArrow` becomes `Flat`; `NotComputable` would be more honest.
- **Re-login on any error:** an authenticated request that fails for any reason (including a 5xx) triggers a fresh login and one retry. Combined with the next point, outages can turn into extra logins.
- **Login per sync:** we build a new client per sync, so every sync logs in again. The token (`authTicket.expires`/`duration`) is never reused. Fine at a 10-hour interval; reusing the token would cut lockout risk if syncs become frequent.
