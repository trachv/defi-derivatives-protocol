/// Fixed-point scaling factor (e.g., 6 decimal places)
const SCALE: u128 = 1_000_000;

/// Calculates an approximate Black-Scholes option price using fixed-point arithmetic
pub fn black_scholes_approx(
    s: u64,     // Current price of the asset (in smallest units)
    k: u64,     // Strike price (in smallest units)
    t: u64,     // Time to expiration (in seconds)
    r: u64,     // Risk-free rate (scaled by 1e6, e.g., 5% -> 500000)
    sigma: u64, // Volatility (scaled by 1e6)
) -> u64 {
    // Convert input parameters to fixed-point numbers
    let s_fp = s as u128 * SCALE;
    let k_fp = k as u128 * SCALE;
    let t_fp = t as u128 * SCALE / 31_536_000; // Convert seconds to years (approximate)
    let r_fp = r as u128;
    let sigma_fp = sigma as u128;

    // Calculate d1 and d2 using fixed-point arithmetic
    // d1 = [ln(s / k) + (r + sigma^2 / 2) * t] / (sigma * sqrt(t))
    // d2 = d1 - sigma * sqrt(t)

    let ln_s_div_k = ln_fp((s_fp * SCALE) / k_fp); // ln(s / k)
    let sigma_squared = (sigma_fp * sigma_fp) / SCALE;
    let half_sigma_squared = sigma_squared / 2;
    let r_plus_half_sigma_squared = r_fp + half_sigma_squared;

    let numerator = ln_s_div_k + (r_plus_half_sigma_squared * t_fp) / SCALE;
    let sigma_sqrt_t = (sigma_fp * sqrt_fp(t_fp)) / SCALE;
    if sigma_sqrt_t == 0 {
        // Avoid division by zero
        return 0;
    }
    let d1 = (numerator * SCALE) / sigma_sqrt_t;
    let d2 = d1 - sigma_sqrt_t;

    // Calculate N(d1) and N(d2)
    let nd1 = standard_normal_cdf(d1);
    let nd2 = standard_normal_cdf(d2);

    // Calculate call option price: C = S * N(d1) - K * e^{-r * t} * N(d2)
    let s_nd1 = (s_fp * nd1) / SCALE;
    let r_t = (r_fp * t_fp) / SCALE;
    let e_minus_rt = exp_fp(SCALE - r_t); // e^{-r * t}
    let k_e_minus_rt = (k_fp * e_minus_rt) / SCALE;
    let k_e_minus_rt_nd2 = (k_e_minus_rt * nd2) / SCALE;

    let c_fp = if s_nd1 >= k_e_minus_rt_nd2 {
        s_nd1 - k_e_minus_rt_nd2
    } else {
        0
    };

    // Convert fixed-point result back to u64
    (c_fp / SCALE) as u64
}

/// Fixed-point natural logarithm approximation: ln(x)
fn ln_fp(x_fp: u128) -> u128 {
    // Using a simple series expansion for ln(x) around x = SCALE (ln(1) = 0)
    // ln(x) ≈ (x - SCALE) / SCALE for x close to SCALE
    let delta = x_fp - SCALE;
    let ln_x_fp = (delta * SCALE) / SCALE; // Simplified to delta
    ln_x_fp
}

/// Fixed-point exponential function approximation: e^{x}
fn exp_fp(x_fp: u128) -> u128 {
    // Using a simple series expansion e^{x} ≈ 1 + x + x^2/2! + x^3/3!
    let x1 = x_fp;
    let x2 = (x_fp * x_fp) / SCALE;
    let x3 = (x2 * x_fp) / SCALE;

    let term1 = SCALE;            // 1
    let term2 = x1;               // x
    let term3 = x2 / 2;           // x^2 / 2!
    let term4 = x3 / 6;           // x^3 / 3!

    let e_x_fp = term1 + term2 + term3 + term4;
    e_x_fp
}

/// Fixed-point square root approximation: sqrt(x)
fn sqrt_fp(x_fp: u128) -> u128 {
    // Using the Babylonian method for square roots
    if x_fp == 0 {
        return 0;
    }
    let mut z = x_fp;
    let mut y = (x_fp + SCALE) / 2;
    while y < z {
        z = y;
        y = ((x_fp * SCALE) / y + y) / 2;
    }
    z
}

/// Standard normal cumulative distribution function approximation: N(d)
fn standard_normal_cdf(d_fp: u128) -> u128 {
    // Using an approximation of the error function
    // N(d) ≈ 0.5 * [1 + erf(d / sqrt(2))]
    // For simplicity, we'll use a linear approximation
    // N(d) ≈ 0.5 + d / (SCALE * sqrt(2 * PI))
    const SQRT_2_PI: u128 = 2_506_628; // sqrt(2 * pi) * SCALE
    let nd_fp = (d_fp * SCALE) / SQRT_2_PI;
    let nd_fp = (SCALE / 2) + nd_fp;
    if nd_fp > SCALE {
        SCALE
    } else if nd_fp < 0 {
        0
    } else {
        nd_fp
    }
}
