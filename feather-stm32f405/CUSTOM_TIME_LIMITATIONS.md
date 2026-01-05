# Custom Time Function Limitations

This document describes the limitations of the custom Unix timestamp ↔ RTC DateTime conversion functions in `src/time.rs`.

## Summary

**Current Implementation**: ~92 lines of custom calendar math  
**Binary Size**: Saves ~12.6 KB vs chrono crate  
**Accuracy**: Good enough for NTP time synchronization (2024-2099)  
**Risk Level**: ⚠️ LOW for current use case

---

## What Works Correctly ✅

### 1. Leap Year Calculation
```rust
fn is_leap_year(year: u16) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}
```
- ✅ Correctly implements Gregorian calendar rules
- ✅ Handles century years (1900, 2000, 2100, etc.)
- ✅ Examples:
  - 2000: leap (divisible by 400)
  - 1900: NOT leap (divisible by 100 but not 400)
  - 2024: leap (divisible by 4, not by 100)
  - 2100: NOT leap (divisible by 100 but not 400)

### 2. Month Day Counts
- ✅ Correctly uses 28/29 days for February based on leap year
- ✅ Correctly handles all other months (30/31 days)

### 3. Unix Timestamp Range
- ✅ Uses `u64` for Unix timestamps (no Year 2038 problem)
- ✅ Theoretically valid until year ~292,277,026,596
- ✅ Practical limit: 2105 (see limitations below)

### 4. Time of Day
- ✅ Hour/minute/second conversions are exact
- ✅ No rounding errors in time calculations

---

## Known Limitations ⚠️

### 1. **Year Range: 1970-2105** 🔴 CRITICAL

**Problem**: Embassy STM32 RTC `DateTime::year()` returns `u16`  
**Range**: 0-65,535  
**Practical Range**: 1970-2105 (u16 can't store years > 65535)

**Impact**:
- ❌ Timestamps for dates after **2105** will overflow or fail
- ❌ `DateTime::from()` will return `Err` for year > u16::MAX
- ✅ Current NTP servers will work until ~2099
- ✅ Device lifetime likely < 2105

**Test Case That Would Fail**:
```rust
let year_2106 = 4294967296u64; // 2106-01-01 00:00:00
let dt = unix_to_datetime(year_2106); // Will fail or wrap
```

**When to Upgrade**: If device needs to operate past 2100.

---

### 2. **Day of Week Always Wrong** 🟡 MINOR

**Problem**: Hardcoded to `DayOfWeek::Monday`

```rust
DateTime::from(
    year, month, day,
    DayOfWeek::Monday,  // ⚠️ Always wrong!
    hour, minute, second, 0
)
```

**Impact**:
- ✅ No impact on timekeeping or NTP sync
- ✅ RTC continues counting correctly
- ❌ `DateTime.day_of_week()` will be incorrect
- ❌ Can't use for calendar display

**When to Upgrade**: If you need to display or use day-of-week information.

**Fix**: Implement [Zeller's Congruence](https://en.wikipedia.org/wiki/Zeller%27s_congruence) or use chrono.

---

### 3. **No Leap Second Support** 🟢 ACCEPTABLE

**Problem**: Doesn't handle [leap seconds](https://en.wikipedia.org/wiki/Leap_second)

**Impact**:
- ✅ NTP/SNTP also ignore leap seconds (UTC-SLS)
- ✅ Most systems (including Linux) ignore them
- ⚠️ Timestamps may be off by ~27 seconds vs TAI (atomic time)
- ✅ OK for logging, monitoring, and most applications

**When to Upgrade**: If you need TAI (atomic time) or GPS time.

---

### 4. **No Timezone Support** 🟢 ACCEPTABLE

**Problem**: All times are UTC only

**Impact**:
- ✅ Perfect for NTP synchronization (always UTC)
- ✅ Perfect for logging (UTC is standard)
- ❌ Can't display local time
- ❌ Can't handle DST (daylight saving time)

**When to Upgrade**: If you need to display times in local timezones.

---

### 5. **Performance: O(n) Year Iteration** 🟡 MINOR

**Problem**: Both conversion functions iterate through years one-by-one

```rust
for y in UNIX_EPOCH_YEAR..dt.year() {
    days += if is_leap_year(y) { 366 } else { 365 };
}
```

**Impact**:
- ✅ Fast for near-term dates (2024-2030): ~60 iterations
- ⚠️ Slower for far-future dates (year 2100): ~130 iterations  
- ⚠️ Very slow for year 2500+: ~530 iterations
- ✅ Still negligible on STM32F405 @ 168 MHz

**When to Upgrade**: If you frequently convert dates far in the future (2200+).

**Fix**: Use lookup tables or mathematical formulas (chrono does this).

---

### 6. **No Input Validation** 🟡 MINOR

**Problem**: Doesn't validate DateTime inputs

**Examples of Invalid Inputs Not Caught**:
- February 30th
- Month 13
- Hour 25
- Negative years

**Impact**:
- ✅ NTP-synced times are always valid
- ⚠️ Manual RTC manipulation could create invalid dates
- ⚠️ Invalid dates may produce incorrect Unix timestamps

**When to Upgrade**: If RTC can be set manually by users.

---

### 7. **Microsecond Precision Lost** 🟢 ACCEPTABLE

**Problem**: RTC only has 1-second resolution

```rust
// Microseconds are discarded when writing to RTC
Ok(Timestamp::new(unix_secs, 0))  // micros always 0
```

**Impact**:
- ✅ Expected limitation of STM32 internal RTC hardware
- ✅ NTP sync preserves microseconds in initial sync
- ⚠️ Between syncs, only 1-second resolution available
- ✅ Acceptable for most logging/monitoring

**When to Upgrade**: If you need sub-second timestamps between NTP syncs (would require external RTC chip).

---

## Test Cases

### Currently Passing ✅
```rust
// Leap years
assert!(is_leap_year(2000));  // Divisible by 400
assert!(is_leap_year(2024));  // Divisible by 4
assert!(!is_leap_year(1900)); // Divisible by 100, not 400
assert!(!is_leap_year(2023)); // Not divisible by 4

// NTP epoch
let ts = Timestamp::from_ntp(NTP_UNIX_OFFSET, 0);
assert_eq!(ts.unix_secs, 0); // 1970-01-01 00:00:00
```

### Edge Cases to Watch 🔍
```rust
// Year 2100 (NOT a leap year)
let feb_29_2100 = unix_to_datetime(4107542400); // 2100-02-29
// This date doesn't exist! Current code will calculate it incorrectly.

// Year 2105+ (will fail or wrap)
let year_2106 = unix_to_datetime(4294967296);
// DateTime::from() will return Err (year > u16::MAX)

// Far future (slow but correct)
let year_3000 = unix_to_datetime(32503680000);
// Takes ~1030 loop iterations but produces correct result
```

---

## When to Switch to Chrono or time Crate

### Immediate Red Flags 🔴
Switch if you need:
1. **Dates after 2105** (u16 year overflow)
2. **Date parsing** ("2024-01-05T10:30:00Z" → timestamp)
3. **Timezone conversions** (UTC → EST/PST/etc.)
4. **Day of week calculations** for display
5. **Date arithmetic** (add 30 days, subtract 2 months, etc.)

### Future Considerations 🟡
Consider switching if:
1. **Performance matters** for far-future dates (2200+)
2. **Manual date input** requires validation
3. **Compliance requirements** mandate battle-tested libraries
4. **Team prefers** industry-standard solutions

### Keep Custom If ✅
1. ✅ Only need NTP sync (current use case)
2. ✅ Dates are 2024-2099
3. ✅ UTC only (no timezones)
4. ✅ 1-second resolution is acceptable
5. ✅ Binary size matters (saves 12.6 KB)

---

## Size Comparison

| Implementation | Binary Size | Lines of Code | Capabilities |
|----------------|-------------|---------------|-------------|
| **Custom** (current) | Baseline | 92 lines | Basic Unix ↔ DateTime |
| **chrono** (no_std) | +12.6 KB | 0 lines | Full date/time library |
| **time** crate | ~+8 KB (estimated) | 0 lines | Lighter than chrono |

---

## Migration Path

If you need to upgrade:

### Option 1: chrono (Full Featured)
```toml
[dependencies]
chrono = { version = "0.4", default-features = false }
```
- ✅ Battle-tested
- ✅ Comprehensive features
- ❌ +12.6 KB binary size
- See: `test/chrono-size-comparison` branch

### Option 2: time crate (Lightweight)
```toml
[dependencies]
time = { version = "0.3", default-features = false }
```
- ✅ Lighter than chrono (~8 KB estimated)
- ✅ Modern API
- ⚠️ Not tested yet

### Option 3: Improve Custom
- Add day-of-week calculation (Zeller's Congruence)
- Add input validation
- Use lookup tables for performance
- Extend year range (requires embassy-stm32 change)

---

## Conclusion

**Current Status**: ✅ **Production Ready** for NTP time sync use case

**Recommendation**: Keep custom implementation unless specific limitations impact your use case.

**Review Date**: Before 2099 or when requirements change.

---

## References

- [RFC 5905 - NTPv4 Specification](https://tools.ietf.org/html/rfc5905)
- [Gregorian Calendar Rules](https://en.wikipedia.org/wiki/Gregorian_calendar)
- [Unix Time](https://en.wikipedia.org/wiki/Unix_time)
- [Leap Seconds](https://en.wikipedia.org/wiki/Leap_second)
- [Year 2038 Problem](https://en.wikipedia.org/wiki/Year_2038_problem)
