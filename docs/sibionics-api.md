# SiBionics / Sisensing cloud API (unofficial notes)

Reverse-engineered by the community; **not documented by SiBionics**. Everything here can change without notice.
Status: learned from two open-source uploaders, **not yet verified against our own account**.

Sources:
- [ImLovinIt/nightscout-sisensingcgm-uploader](https://github.com/ImLovinIt/nightscout-sisensingcgm-uploader) (MIT). Read from its Docker image `imlovinit1019/nightscout-sisensingcgm-uploader:latest` (`package/module.py`, `package/setup.py`) and its README / `sample_json/`.
- [Lewis5920/sisensing-nightscout-uploader](https://github.com/Lewis5920/sisensing-nightscout-uploader) (no license stated, so treat as reference only).

## Auth

- Header: `Authorization: Bearer <token>`.
- **No login or refresh endpoint is known.** Both projects tell users to capture the token from the official app's traffic (HTTP Toolkit, Fiddler, packet-capture app) and paste it in.
- Lifetime is not documented. It is known to be invalidated by reinstalling the app or logging in on another device with the same account. A `401` means re-capture.
- ImLovinIt sends only `Authorization`, `Content-Type: application/json`, `Accept: application/json` and its own `User-Agent`, so no app-version or device headers appear to be required on `api.sisensing.com`.

## Hosts

| Host | Used by | Notes |
| ---- | ------- | ----- |
| `api.sisensing.com` | ImLovinIt (own data, app) | Only tested with Chinese accounts (needs a Chinese mobile number) |
| `cxm-api.sisensing.com` | Lewis5920 (friend share, WeChat mini program) | China; needs WeChat-specific headers |

**Unknown:** the host used by the international/EU SiBionics app (GS1 sold in Europe). It is probably different, and is the first thing to capture.

## Endpoints

All `GET`, all return the envelope below.

### Own data, 14 days

```
GET https://api.sisensing.com/follow/app/follow/myself/glucose/details/devices
```

Device blocks at `data.glucoseDataList[]`.

### Own data, 18 months

```
GET https://api.sisensing.com/lite-sense-app/follow/myself/glucose/
```

(trailing slash as documented). Device block at `data.followedDeviceGlucoseDataPO`, alongside follow metadata (`followedUserId`, `followedUserInfo{userName,nickName,...}`, `followTime`, ...).

### Friend share (WeChat mini program)

```
GET https://cxm-api.sisensing.com/cxm-mini-program-share/follow/relation/data/details-v2
    ?appCode=SIJOY_HEALTH_APP&followRelationId=<id>&deviceId=<id>
```

Extra headers: `app_code: SHARE_MINI_PROGRAM`, `sib-agent: SHARE_MINI_PROGRAM&01.01.00.00&<os>&NULL&<vendor>`, `timezone: Asia/Shanghai`, plus a WeChat user agent/referer. Readings at `data.deviceGlucoseInfos[].glucoseInfos[]`, latest at `data.deviceInfo.latestValue/latestTime`. Different shape from the two above; not a good fit for us.

## Response

```json
{
  "timestamp": 1735214700000,
  "code": 200,
  "msg": "成功",
  "data": { "...": "see endpoint" }
}
```

`code != 200` means failure (bad token, bad URL). No list of error codes is known.

### Device block

| Field | Meaning |
| ----- | ------- |
| `deviceId`, `userId`, `deviceName` | identifiers |
| `deviceStatus` | `1` = sensor running; anything else = ended |
| `deviceEnableTime` | activation, **in seconds**, and skewed (reads like a UTC+8 wall clock). Don't use it |
| `latestGlucoseValue`, `latestGlucoseTime` | latest reading (mmol/L, ms; time may be a string) |
| `deviceLastTime` | activation + 14 days (schedule, not an observation); ms, may be a string |
| `bloodGlucoseTrend` | latest trend, same scale as `s` |
| `glucoseInfos[]` | readings, see below |
| `dailyData[]` | per-day summaries `{dateTime, tirScale, data:[{t,v,effective}]}` |

### Reading (`glucoseInfos[]`)

```json
{"t":1735214700472,"v":4.0,"i":17455,"s":-1,"ast":1,"bl":2950.0,"effective":true}
```

| Field | Meaning |
| ----- | ------- |
| `t` | timestamp, epoch **milliseconds** (UTC) |
| `v` | glucose in **mmol/L**; mg/dL = `v * 18.018` (ImLovinIt) or `v * 18` (Lewis) |
| `s` | trend: `-2` SingleDown, `-1` FortyFiveDown, `0` Flat, `1` FortyFiveUp, `2` SingleUp |
| `i` | minutes since sensor activation (steps of 5); `t - i*60000` gives the true activation |
| `effective` | presumably whether the reading is valid. Unconfirmed |
| `ast`, `bl` | unknown |

One reading every 5 minutes, available about 5 minutes late.

## Mapping to our model

| Ours | From |
| ---- | ---- |
| `value_mg_dl` | `v * 18.018`, rounded |
| `timestamp` | `t` (ms) |
| `trend` | `s`: `-2` singleDown, `-1` fortyFiveDown, `0` flat, `1` fortyFiveUp, `2` singleUp, anything else notComputable |
| `is_high` / `is_low` | not provided; needs thresholds or leave false |
| `device_id` | `deviceId` |

Skip readings with `effective: false`.

## Open questions (verify with a capture from our own account)

1. Host and paths used by the international/EU app for a GS1.
2. Whether the international app has a login endpoint we can call with email + password (would let us store credentials like LibreLink instead of a captured token).
3. Token lifetime, and what an expired token returns (`401` vs `code != 200`).
4. Meaning of `effective`, `ast`, `bl`.
