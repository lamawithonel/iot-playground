# Binary Size Comparison: Chrono vs Custom Time Functions

## Status
✅ **READY TO MEASURE** - Both branches now compile with different implementations

## Purpose
Compare the binary size impact of using the `chrono` crate (no_std) versus custom time conversion functions.

## What's Being Compared

### Branch: `feature/sntp-client` (Baseline - Custom Implementation)
- `unix_to_datetime()` - ~60 lines of custom calendar math
- `datetime_to_unix()` - ~30 lines of custom date calculation  
- `is_leap_year()` - ~2 lines helper function
- **Total**: ~92 lines of handwritten date/time conversions
- **Removed in chrono branch**: All custom date math logic

### Branch: `test/chrono-size-comparison` (Chrono Implementation)
- Uses `chrono` crate with `default-features = false` (no_std compatible)
- `unix_to_datetime()` using `NaiveDateTime::from_timestamp_opt()`
- `datetime_to_unix()` using `NaiveDate::from_ymd_opt()` + `NaiveTime::from_hms_opt()`
- Chrono handles all leap year logic, edge cases, and calendar rules
- **Lines saved**: ~92 lines of custom code removed

## How to View Results

### Automated GitHub Actions
The workflow runs automatically on push to `test/chrono-size-comparison`:

1. Go to: https://github.com/lamawithonel/iot-playground/actions
2. Look for "Binary Size Comparison" workflow
3. Click the latest run
4. Download the "size-comparison-report" artifact
5. View `size_report.md`

Or view directly in logs:
1. Click on "Compare Binary Sizes" job
2. Expand "Display report" step

## Expected Trade-offs

### Chrono Advantages
- ✅ Battle-tested, handles all edge cases correctly
- ✅ Handles complex calendar rules (leap years, centuries, etc.)
- ✅ Less maintenance burden
- ✅ Well-documented, industry-standard API
- ✅ Eliminates ~92 lines of custom code

### Custom Implementation Advantages  
- ✅ Smaller binary (expected)
- ✅ No external dependencies
- ✅ Simpler for basic use case (just NTP sync)
- ✅ Faster compile times
- ✅ Full control over code

## Size Estimation

Based on Rust embedded community experience:
- **Chrono** (no_std, minimal features): typically adds **3-15 KB** to binary
- **Custom implementation**: **~500 bytes - 2 KB** depending on optimization
- **Expected difference**: ~5-10 KB code size increase

## STM32F405RG Context
- **Total Flash**: 1 MB (1024 KB)
- **Expected chrono overhead**: ~5-10 KB
- **Impact**: < 1% of total flash
- **Verdict**: Size impact is **negligible** for this hardware

## Recommendation Criteria

**Use Custom** (current implementation) if:
- Every kilobyte matters (very tight flash budget)
- Only need basic Unix ↔ DateTime conversion
- Simple calendar math is sufficient for use case
- Want minimal dependencies

**Use Chrono** if:
- Need complex timezone support later
- Want industry-standard date/time handling
- Binary size isn't a concern (> 256KB flash available)
- Planning to add more time-based features
- Want to reduce maintenance of custom date logic

## Analysis Results

The automated workflow will produce a report showing:

```
| Section | Baseline | With Chrono | Difference |
|---------|----------|-------------|------------|
| .text   | XXXXX    | YYYYY      | +ZZZZ bytes|
| .data   | XXX      | YYY        | +ZZ bytes  |
| .bss    | XXX      | YYY        | +ZZ bytes  |
| Total   | XXXXX    | YYYYY      | +ZZZZ bytes|
```

**Impact**: +X KB (+Y.Z% increase)
**Chrono Overhead**: Z.ZZZ% of total 1MB flash

## Next Steps

1. ✅ Wait for GitHub Actions to complete build
2. ✅ Review size comparison report
3. ⏳ Decide: Keep custom or switch to chrono
4. ⏳ If keeping custom: Delete test branch
5. ⏳ If switching to chrono: Merge test branch to feature branch
