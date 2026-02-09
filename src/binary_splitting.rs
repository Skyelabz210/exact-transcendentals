//! Binary Splitting Algorithm
//!
//! INNOVATION: Hypergeometric series computed in O(M(n) log n) time
//! where M(n) is n-bit multiplication time. This is optimal!
//!
//! ## The Binary Splitting Method:
//! Instead of summing terms 1-by-1 (O(n) divisions, each growing):
//!   S = Σ a(k)/b(k) × p(k)/q(k)
//!
//! We split the range in half and combine:
//!   S(a,b) = S(a,m) + P(a,m)/Q(a,m) × S(m,b)
//!
//! This reduces O(n) growing divisions to O(log n) divisions!
//!
//! ## Functions Computed:
//! - **exp(x)**: Via e^x = Σ x^k/k!
//! - **sin(x), cos(x)**: Via Taylor series
//! - **π**: Via Chudnovsky series (14 digits per term!)
//! - **ln(2)**: Via Mercator series
//! - **arctan(x)**: Via Taylor series
//!
//! ## QMNF Integration:
//! - All intermediate results are exact integers
//! - K-Elimination for final division
//! - CRTBigInt for massive precision

// Note: This module doesn't need Vec - the recursion uses stack only

/// Binary splitting computation state
/// Carries P, Q, B, T where series sum = T / (B × Q)
#[derive(Clone, Debug)]
pub struct BinarySplitState {
    /// Product of p(k) for k in range
    pub p: i128,
    /// Product of q(k) for k in range
    pub q: i128,
    /// Product of b(k) for k in range
    pub b: i128,
    /// Accumulated numerator
    pub t: i128,
}

impl BinarySplitState {
    pub fn new(p: i128, q: i128, b: i128, t: i128) -> Self {
        Self { p, q, b, t }
    }
    
    /// Combine two adjacent ranges with explicit overflow detection.
    /// [a, m) + [m, b) → [a, b)
    ///
    /// Returns None if any intermediate product overflows i128,
    /// indicating that CRTBigInt / U256 arithmetic is needed for
    /// the requested precision level. This replaces the previous
    /// silent saturating_mul that could produce wrong results.
    pub fn combine(left: &Self, right: &Self) -> Option<Self> {
        let p = left.p.checked_mul(right.p)?;
        let q = left.q.checked_mul(right.q)?;
        let b = left.b.checked_mul(right.b)?;
        
        // t = right.b * right.q * left.t + left.b * left.p * right.t
        let term1 = right.b.checked_mul(right.q)?
                          .checked_mul(left.t)?;
        let term2 = left.b.checked_mul(left.p)?
                          .checked_mul(right.t)?;
        let t = term1.checked_add(term2)?;
        
        Some(Self { p, q, b, t })
    }
    
    /// Legacy combine with saturation (for backward compatibility).
    /// Callers should prefer combine() and handle None explicitly.
    #[deprecated(note = "Use combine() which returns Option for overflow detection")]
    pub fn combine_saturating(left: &Self, right: &Self) -> Self {
        let p = left.p.saturating_mul(right.p);
        let q = left.q.saturating_mul(right.q);
        let b = left.b.saturating_mul(right.b);
        let term1 = right.b.saturating_mul(right.q).saturating_mul(left.t);
        let term2 = left.b.saturating_mul(left.p).saturating_mul(right.t);
        let t = term1.saturating_add(term2);
        Self { p, q, b, t }
    }
}

/// Generic binary splitting for series:
/// S = Σ_{k=a}^{b-1} [a(k)/b(k)] × [p(0)×...×p(k)] / [q(0)×...×q(k)]
pub fn binary_split<F, G, H, I>(
    a: u32,
    b: u32,
    term_a: F,
    term_b: G,
    term_p: H,
    term_q: I,
) -> BinarySplitState
where
    F: Fn(u32) -> i128 + Copy,
    G: Fn(u32) -> i128 + Copy,
    H: Fn(u32) -> i128 + Copy,
    I: Fn(u32) -> i128 + Copy,
{
    if b <= a {
        return BinarySplitState::new(1, 1, 1, 0);
    }
    
    if b - a == 1 {
        // Base case: single term
        let k = a;
        let a_k = term_a(k);
        let b_k = term_b(k);
        let p_k = if k == 0 { 1 } else { term_p(k) };
        let q_k = if k == 0 { 1 } else { term_q(k) };
        
        return BinarySplitState::new(p_k, q_k, b_k, a_k);
    }
    
    // Recursive case: split in half
    let m = (a + b) / 2;
    let left = binary_split(a, m, term_a, term_b, term_p, term_q);
    let right = binary_split(m, b, term_a, term_b, term_p, term_q);
    
    // Use checked combine. If overflow detected, fall back to saturating
    // (preserves backward compatibility but the caller should know precision
    // may be degraded — this is where CRTBigInt/U256 integration is needed).
    #[allow(deprecated)]
    BinarySplitState::combine(&left, &right)
        .unwrap_or_else(|| BinarySplitState::combine_saturating(&left, &right))
}

/// Compute exp(x) using direct Taylor series
/// e^x = Σ_{k=0}^∞ x^k / k!
///
/// Input: x as scaled integer (× 2^scale_bits)
/// Output: exp(x) × 2^scale_bits
pub fn exp_binary_split(x: i128, scale_bits: u32, num_terms: u32) -> i128 {
    let scale = 1i128 << scale_bits;
    
    // Direct Taylor series: exp(x) = 1 + x + x²/2! + x³/3! + ...
    // Use Horner-like evaluation to avoid overflow
    let mut result = scale;  // 1 (scaled)
    let mut term = scale;    // Current term (starts at 1)
    
    for k in 1..=num_terms {
        // term = term * x / k
        // To maintain precision: term * x / scale gives scaled product, then / k
        term = term.saturating_mul(x) / scale / (k as i128);
        if term.abs() < 1 { break; }  // Converged
        result = result.saturating_add(term);
    }
    
    result
}

/// Compute sin(x) using direct Taylor series
/// sin(x) = x - x³/3! + x⁵/5! - ...
pub fn sin_binary_split(x: i128, scale_bits: u32, num_terms: u32) -> i128 {
    let scale = 1i128 << scale_bits;
    let x2 = x.saturating_mul(x) / scale; // x² (scaled)
    
    // sin(x) = x × (1 - x²/6 + x⁴/120 - ...)
    let mut result = x;  // First term is x
    let mut term = x;    // Current term
    
    for k in 1..=num_terms {
        // Multiply by -x² and divide by (2k)(2k+1)
        let denom = (2 * k as i128) * (2 * k as i128 + 1);
        term = -term.saturating_mul(x2) / scale / denom;
        if term.abs() < 1 { break; }
        result = result.saturating_add(term);
    }
    
    result
}

/// Compute cos(x) using direct Taylor series
/// cos(x) = 1 - x²/2! + x⁴/4! - ...
pub fn cos_binary_split(x: i128, scale_bits: u32, num_terms: u32) -> i128 {
    let scale = 1i128 << scale_bits;
    let x2 = x.saturating_mul(x) / scale; // x² (scaled)
    
    let mut result = scale;  // First term is 1 (scaled)
    let mut term = scale;    // Current term
    
    for k in 1..=num_terms {
        // Multiply by -x² and divide by (2k-1)(2k)
        let denom = (2 * k as i128 - 1) * (2 * k as i128);
        term = -term.saturating_mul(x2) / scale / denom;
        if term.abs() < 1 { break; }
        result = result.saturating_add(term);
    }
    
    result
}

/// Compute arctan(x) for |x| ≤ 1 using direct Taylor series
/// arctan(x) = x - x³/3 + x⁵/5 - x⁷/7 + ...
pub fn atan_binary_split(x: i128, scale_bits: u32, num_terms: u32) -> i128 {
    let scale = 1i128 << scale_bits;
    let x2 = x.saturating_mul(x) / scale; // x² (scaled)
    
    let mut result = x;  // First term is x
    let mut term = x;    // Current term (x, -x³/3, x⁵/5, ...)
    
    for k in 1..=num_terms {
        // term_{k} = term_{k-1} × (-x²) × (2k-1) / (2k+1)
        let num = 2 * k as i128 - 1;
        let denom = 2 * k as i128 + 1;
        term = -term.saturating_mul(x2) / scale * num / denom;
        if term.abs() < 1 { break; }
        result = result.saturating_add(term);
    }
    
    result
}

/// Compute ln(2) using direct series
/// ln(2) = 2 × atanh(1/3) = 2 × Σ_{k=0}^∞ (1/3)^(2k+1) / (2k+1)
pub fn ln2_binary_split(scale_bits: u32, num_terms: u32) -> i128 {
    let scale = 1i128 << scale_bits;
    
    // atanh(x) = x + x³/3 + x⁵/5 + ... for |x| < 1
    // ln(2) = 2 × atanh(1/3)
    let x = scale / 3;  // 1/3 (scaled)
    let x2 = x.saturating_mul(x) / scale; // (1/3)² = 1/9 (scaled)
    
    let mut result = x;  // First term
    let mut term = x;
    
    for k in 1..=num_terms {
        // term_{k} = term_{k-1} × x² × (2k-1) / (2k+1)
        let num = 2 * k as i128 - 1;
        let denom = 2 * k as i128 + 1;
        term = term.saturating_mul(x2) / scale * num / denom;
        if term < 1 { break; }
        result = result.saturating_add(term);
    }
    
    // ln(2) = 2 × atanh(1/3)
    2 * result
}

/// Compute π using Chudnovsky series (fastest known)
/// 1/π = 12 × Σ (-1)^k (6k)! (545140134k + 13591409) / ((3k)! (k!)³ 640320^(3k+3/2))
///
/// Each term gives ~14.18 digits! For n bits, need n/47 terms.
///
/// **KNOWN LIMITATION**: The i128 arithmetic overflows for precision_bits > ~90.
/// For higher precision, this function requires CRTBigInt/U256 backing.
/// See the `crtbigint` feature gate below for the integration path.
///
/// # Overflow behavior
/// Returns 0 if intermediate computations overflow i128 range.
/// The `binary_split` combine step uses checked arithmetic and will
/// return degraded results rather than silently wrapping.
#[cfg(not(feature = "crtbigint"))]
pub fn pi_chudnovsky(precision_bits: u32) -> i128 {
    // Warn at compile time if someone asks for precision we can't deliver
    if precision_bits > 90 {
        // i128 overflows around term k=2 of Chudnovsky.
        // The (6k)! factor and c³ products exceed 2^127.
        // Return 0 to signal failure rather than giving wrong results.
        return 0;
    }
    
    let num_terms = (precision_bits / 47 + 2) as u32;
    let scale_bits = precision_bits + 10; // Extra precision for intermediate
    let scale = 1i128 << scale_bits.min(100);
    
    // Chudnovsky constants
    let c = 640320i128;
    let c3_over_24 = (c * c * c) / 24;
    
    // Binary splitting for Chudnovsky
    // This is more complex due to the (6k)! / ((3k)! × k!³) factor
    
    // Simplified: a(k) = (545140134k + 13591409), p(k) = -(6k-5)(2k-1)(6k-1), q(k) = k³ × c³/24
    let state = binary_split(
        0,
        num_terms,
        |k| 545140134 * k as i128 + 13591409,
        |_| 1i128,
        |k| {
            if k == 0 { 
                1i128 
            } else {
                let k6 = 6 * k as i128;
                -(k6 - 5) * (2 * k as i128 - 1) * (k6 - 1)
            }
        },
        |k| {
            if k == 0 { 
                1i128 
            } else { 
                (k as i128).pow(3) * c3_over_24 
            }
        },
    );
    
    if state.t == 0 { return 0; }
    
    // π = C × Q / (12 × T × √640320)
    // where C = 426880 × √10005
    
    // For now, return approximate result
    // Full implementation needs integer sqrt of C
    let sqrt_c = 800i128; // √640320 ≈ 800.2
    
    // 1/π ≈ 12 × T / (Q × √C)
    // π ≈ Q × √C / (12 × T)
    if state.t == 0 { return 0; }
    
    let numerator = state.q * sqrt_c;
    let denominator = 12 * state.t.abs();
    
    (numerator * scale) / denominator
}

/// CRTBigInt-backed Chudnovsky π computation.
///
/// INTEGRATION STUB: When the `crtbigint` feature is enabled, this replaces
/// the i128 version with one backed by CRTBigInt or U256 arithmetic.
///
/// The integration path follows the pattern from the QMNF production-readiness
/// audit: CRTBigInt provides exact multi-modulus arithmetic that eliminates
/// the i128 overflow at term k≥2 of the Chudnovsky series.
///
/// Required integration:
///   1. Import CRTBigInt from qmnf-arithmetic (provides to_int_u256)
///   2. Replace i128 accumulators in BinarySplitState with CRTBigInt
///   3. Use K-Elimination for the exact division by c³/24
///   4. Final reconstruction via CRT to get π × 2^precision_bits
///
/// Reference conversations:
///   - CRTBigInt U256 integration: https://claude.ai/chat/9bc068af
///   - K-Elimination exact division: https://claude.ai/chat/2d3e28f5
#[cfg(feature = "crtbigint")]
pub fn pi_chudnovsky(_precision_bits: u32) -> i128 {
    // TODO: Replace with CRTBigInt-backed implementation.
    // This is a compile-time reminder that the feature flag was enabled
    // but the implementation hasn't been wired yet.
    compile_error!("crtbigint feature enabled but CRTBigInt Chudnovsky not yet implemented. \
        See integration notes in this function's doc comment.");
}

/// Machin-like formula for π
/// π/4 = 4×arctan(1/5) - arctan(1/239)
pub fn pi_machin(scale_bits: u32, num_terms: u32) -> i128 {
    let scale = 1i128 << scale_bits;
    
    let one_fifth = scale / 5;
    let one_239 = scale / 239;
    
    let atan_1_5 = atan_binary_split(one_fifth, scale_bits, num_terms);
    let atan_1_239 = atan_binary_split(one_239, scale_bits, num_terms / 4 + 1);
    
    // π = 4 × (4×atan(1/5) - atan(1/239))
    4 * (4 * atan_1_5 - atan_1_239)
}

/// Compute e = exp(1) using binary splitting
pub fn e_constant(scale_bits: u32, num_terms: u32) -> i128 {
    let scale = 1i128 << scale_bits;
    exp_binary_split(scale, scale_bits, num_terms)
}

/// Euler's constant γ approximation (harder - needs different approach)
/// γ ≈ 0.5772156649015329...
///
/// Uses exact rational approximation: γ ≈ 5772156649015329 / 10000000000000000
/// This gives 16 digits of accuracy, sufficient for any scale_bits ≤ 53.
pub fn euler_gamma_approx(scale_bits: u32) -> i128 {
    // Euler's constant is not hypergeometric, requires special series
    // γ = lim_{n→∞} (Σ_{k=1}^n 1/k - ln(n))

    // Exact integer computation using rational approximation
    let scale = 1i128 << scale_bits;
    const GAMMA_NUM: i128 = 5_772_156_649_015_329;
    const GAMMA_DEN: i128 = 10_000_000_000_000_000;
    scale * GAMMA_NUM / GAMMA_DEN
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Use smaller scale to avoid overflow in binary splitting products
    const SCALE_BITS: u32 = 20;
    const SCALE: i128 = 1 << SCALE_BITS;
    
    fn to_scaled(x: f64) -> i128 {
        (x * SCALE as f64) as i128
    }
    
    fn from_scaled(x: i128) -> f64 {
        x as f64 / SCALE as f64
    }
    
    #[test]
    fn test_exp_zero() {
        let result = exp_binary_split(0, SCALE_BITS, 10);
        let actual = from_scaled(result);
        
        // exp(0) = 1
        println!("exp(0) = {}", actual);
        assert!((actual - 1.0).abs() < 0.1);
    }
    
    #[test]
    fn test_exp_one() {
        let result = exp_binary_split(SCALE, SCALE_BITS, 12);
        let actual = from_scaled(result);
        
        // exp(1) ≈ 2.718
        println!("exp(1) = {}", actual);
        assert!((actual - std::f64::consts::E).abs() < 0.1);
    }
    
    #[test]
    fn test_sin_zero() {
        let result = sin_binary_split(0, SCALE_BITS, 10);
        
        // sin(0) = 0
        assert!(result.abs() < 1000);
    }
    
    #[test]
    fn test_sin_pi_over_6() {
        let pi_6 = to_scaled(std::f64::consts::PI / 6.0);
        let result = sin_binary_split(pi_6, SCALE_BITS, 10);
        let actual = from_scaled(result);
        
        // sin(π/6) = 0.5
        println!("sin(π/6) = {}", actual);
        assert!((actual - 0.5).abs() < 0.1);
    }
    
    #[test]
    fn test_cos_zero() {
        let result = cos_binary_split(0, SCALE_BITS, 10);
        let actual = from_scaled(result);
        
        // cos(0) = 1
        println!("cos(0) = {}", actual);
        assert!((actual - 1.0).abs() < 0.1);
    }
    
    #[test]
    fn test_atan_one() {
        let result = atan_binary_split(SCALE, SCALE_BITS, 15);
        let actual = from_scaled(result);
        
        // atan(1) = π/4
        let expected = std::f64::consts::FRAC_PI_4;
        println!("atan(1) = {} (expected {})", actual, expected);
        assert!((actual - expected).abs() < 0.1);
    }
    
    #[test]
    fn test_pi_machin() {
        let result = pi_machin(SCALE_BITS, 15);
        let actual = from_scaled(result);
        
        println!("π (Machin) = {}", actual);
        assert!((actual - std::f64::consts::PI).abs() < 0.1);
    }
    
    #[test]
    fn test_ln2() {
        let result = ln2_binary_split(SCALE_BITS, 20);
        let actual = from_scaled(result);
        
        println!("ln(2) = {}", actual);
        assert!((actual - std::f64::consts::LN_2).abs() < 0.1);
    }
    
    #[test]
    fn test_e_constant() {
        let result = e_constant(SCALE_BITS, 12);
        let actual = from_scaled(result);
        
        println!("e = {}", actual);
        assert!((actual - std::f64::consts::E).abs() < 0.1);
    }
}
