# Remediation Plan: exact_transcendentals Gap Analysis

## Phase 1: Critical Correctness Fixes
1. Fix CORDIC magnitude() vectoring mode bug (d always -1 after abs())
2. Remove binary_split() fallback to saturating — propagate None upward
3. Remove orphaned `crtbigint` feature, consolidate with `arbitrary-precision`
4. Fix ExactRational debug_assert → assert for zero denominator

## Phase 2: Error Handling Consistency
5. Convert sentinel-value returns to Result<T, TranscendentalError>
6. Add domain checks for CORDIC hyperbolic convergence range
7. Make TranscendentalError Display unconditional (use core::fmt)

## Phase 3: Dead Code and Feature Cleanup
8. Remove dead BigSqrt stub in sqrt.rs (or gate behind arbitrary-precision)
9. Remove empty simd feature flag
10. Fix CORDIC gain factor for non-default iteration counts

## Phase 4: Test Coverage
11. Add tests for untested public functions
12. Add edge case tests for overflow boundaries

## Phase 5: Quality
13. Fix clippy warnings (unnecessary casts)
