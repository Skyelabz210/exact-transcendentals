# Exact Transcendentals

**Zero-floating-point transcendental function computation for the QMNF ecosystem.**

Exact Transcendentals computes logarithms, exponentials, trigonometric functions, π, and algebraic irrationals using exclusively integer arithmetic. Every operation uses only addition, subtraction, multiplication, bit-shift, and integer square root — no floating-point operations anywhere in the computation pipeline.

## Why This Exists

IEEE 754 floating-point arithmetic introduces representational error at every operation. For most applications this is tolerable. For three domains it is not:

- **Fully Homomorphic Encryption (FHE):** Noise budgets are measured in individual bits. A single rounding error can render a ciphertext undecryptable.
- **Post-Quantum Cryptography:** Security proofs assume exact modular arithmetic.
- **Formal Verification:** Any gap between specification and implementation undermines the proof chain.

This library enforces the QMNF invariant: no computation introduces representational error beyond the mathematically unavoidable truncation inherent in finite-precision rational approximation.

## Algorithm Portfolio

| Engine | Functions | Convergence | Precision |
|--------|-----------|-------------|-----------|
| CORDIC | sin, cos, atan, sinh, cosh, exp, ln | Linear (1 bit/iter) | 30-bit (i64) |
| AGM | ln, π, exp, elliptic K, E | Quadratic (2×/iter) | 62-bit (u128) |
| Binary Splitting | π, e, exp, sin, cos, atan | Superlinear | Configurable |
| Continued Fractions | √n, φ, e, π, Pell equations | Quadratic (for surds) | Exact rational |
| Newton-Raphson | √n, ³√n, 1/√n | Quadratic | 30/62/128-bit |

## Quick Start

```rust
use exact_transcendentals::cordic::CordicEngine;
use exact_transcendentals::agm::AgmEngine;

// Trigonometry via CORDIC (30-bit precision)
let cordic = CordicEngine::default();
let (cos, sin) = cordic.sincos(cordic.quarter_pi()); // π/4
// cos ≈ sin ≈ 0.7071 × 2^30

// Logarithm via AGM (62-bit precision)
let agm = AgmEngine::new(20);
let ln_2 = agm.ln(2 * exact_transcendentals::agm::AGM_SCALE);
// ln_2 ≈ 0.6931 × 2^62

// π via Gauss-Legendre (62-bit precision)
let pi = agm.compute_pi_scaled();
// pi ≈ 3.14159 × 2^62
```

## Architecture

```
exact_transcendentals/
├── src/
│   ├── lib.rs                  # Core: ExactRational, binary GCD, error types
│   ├── cordic.rs               # Circular + hyperbolic CORDIC engines
│   ├── agm.rs                  # AGM iteration, ln, π, exp, elliptic integrals
│   ├── binary_splitting.rs     # Hypergeometric series, Chudnovsky π
│   ├── continued_fraction.rs   # CF expansion, convergents, Pell solver
│   ├── sqrt.rs                 # Integer sqrt suite (4 algorithms)
│   └── constants.rs            # Precomputed constants at 30/62-bit precision
└── Cargo.toml
```

## Error Handling

All overflow paths use checked arithmetic via `TranscendentalError`:

```rust
pub enum TranscendentalError {
    Overflow(&'static str),           // Intermediate product exceeds range
    DomainError(&'static str),        // ln(0), sqrt(-1), etc.
    ConvergenceFailure { iterations: u32 },
    SaturationDetected(&'static str), // Would have silently corrupted result
}
```

No silent saturation. No sentinel confusion. Every failure is explicit and typed.

## QMNF Integration Points

- **K-Elimination:** Exact division for AGM arithmetic mean, Chudnovsky splitting, and CF error bounds.
- **CRTBigInt:** Arbitrary-precision extension beyond i128 range (feature-gated).
- **Montgomery Persistence:** 15–20% throughput improvement for AGM multiply chains.
- **Shadow Entropy:** Zero-cost randomness harvesting from CORDIC decision sequences.

## Verification

```bash
# Run all 102 tests
cargo test

# Run specific test categories
cargo test agm::tests           # AGM engine tests
cargo test cordic::tests        # CORDIC engine tests  
cargo test cross_validation     # Cross-engine validation
cargo test identity_tests       # Mathematical identity tests
cargo test truth_perturber      # Truth perturbation discovery tests
```

The test suite includes round-trip identities (exp∘ln = id, sin²+cos² = 1), boundary values (ln(0), exp(0), sincos(0)), cross-engine validation (AGM π vs Machin π vs CF convergent), and 14 truth perturbation tests that probe adjacent mathematical structure.

## Performance Targets

| Operation | Engine | Precision | Estimated Latency |
|-----------|--------|-----------|-------------------|
| sin/cos | CORDIC | 30-bit | <100 ns |
| ln | AGM | 62-bit | <1 μs |
| π | Gauss-Legendre | 62-bit | <1 μs |
| isqrt | Newton-Raphson | 64-bit | <50 ns |

## Building

```bash
# Default (std)
cargo build --release

# No-std (embedded/FHE targets)
cargo build --release --no-default-features

# With CRTBigInt integration (when available)
cargo build --release --features crtbigint
```

## License

MIT OR Apache-2.0

## Citation

```bibtex
@software{exact_transcendentals,
  title  = {Exact Transcendentals: Zero-Floating-Point Transcendental Function Computation},
  author = {HackFate Research},
  year   = {2026},
  url    = {https://hackfate.us}
}
```
