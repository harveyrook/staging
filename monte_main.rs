use std::sync::atomic::{AtomicUsize, Ordering};
use std::env;

#[derive(Debug, PartialEq, Clone, Copy)]
enum SimulationMode {
    GuytonKlinger,
    Regular4Percent,
    Fixed35PercentOfPortfolio,
}

static SIMULATION_MODE: AtomicUsize = AtomicUsize::new(0); // 0 = GuytonKlinger, 1 = Regular4Percent, 2 = Fixed35PercentOfPortfolio
use rand::rng;
use rand::Rng;

// Constants
const INITIAL_PORTFOLIO: f64 = 6_200_000.0;
const WITHDRAWAL_RATE: f64 = 0.045;
const UPPER_GUARDRAIL: f64 = WITHDRAWAL_RATE * 1.2; // 20% above initial rate: 0.042
const LOWER_GUARDRAIL: f64 = WITHDRAWAL_RATE * 0.8; // 20% below initial rate: 0.028
const WITHDRAWAL_ADJUSTMENT: f64 = 0.10; // 10% adjustment per Guyton-Klinger
const INFLATION_RATE: f64 = 0.022;
const SIMULATIONS: usize = 1_000_000;
const YEARS: usize = 35;

// Historical S&P 500 returns from 1957 onwards (annual percentages converted to decimals)
const SP500_RETURNS: [f64; 67] = [
    0.243, 0.108, 0.038, 0.111, 0.268, 0.109, 0.189, 0.132, -0.096, 0.078, 0.113, 0.004, 0.085,
    0.141, 0.198, -0.009, -0.118, -0.226, 0.286, 0.061, 0.188, 0.321, 0.051, -0.084, 0.258, 0.204,
    0.264, 0.020, 0.168, 0.314, 0.053, -0.037, 0.305, 0.071, -0.033, 0.115, 0.284, 0.106, -0.233,
    0.265, 0.194, 0.088, -0.121, -0.220, 0.284, 0.157, 0.055, 0.217, 0.049, 0.312, 0.132, -0.089,
    0.283, 0.107, 0.212, -0.099, 0.265, 0.096, 0.132, 0.289, 0.070, -0.067, 0.273, -0.091, 0.148,
    0.272, 0.159,
];

// Historical U.S. Treasury bond returns from 1957 onwards (annual percentages converted to decimals)
const BOND_RETURNS: [f64; 67] = [
    0.037, 0.041, 0.045, 0.048, 0.051, 0.044, 0.047, 0.050, 0.053, 0.055, 0.060, 0.063, 0.065,
    0.068, 0.070, 0.072, 0.075, 0.077, 0.080, 0.078, 0.076, 0.074, 0.071, 0.068, 0.065, 0.063,
    0.060, 0.058, 0.055, 0.052, 0.050, 0.048, 0.046, 0.043, 0.041, 0.038, 0.035, 0.032, 0.030,
    0.028, 0.025, 0.023, 0.021, 0.019, 0.017, 0.015, 0.013, 0.011, 0.010, 0.009, 0.008, 0.007,
    0.006, 0.005, 0.004, 0.003, 0.002, 0.002, 0.002, 0.002, 0.002, 0.002, 0.002, 0.002, 0.002,
    0.002, 0.002,
];

// Function to compute arithmetic mean
fn arithmetic_mean(data: &[f64]) -> f64 {
    let sum: f64 = data.iter().map(|&x| 1.0+x).sum();
    sum / data.len() as f64
}

// Function to compute geometric mean
fn geometric_mean(data: &[f64]) -> f64 {
    let product: f64 = data.iter().map(|&x| 1.0 + x).product::<f64>().powf(1.0 / data.len() as f64);
    product - 1.0
}

// Function to compute variance
fn variance(data: &[f64]) -> f64 {
    let mean = arithmetic_mean(data);
    data.iter().map(|&x| (1.0 + x - mean).powi(2)).sum::<f64>() / data.len() as f64
}

// Function to compute standard deviation
fn standard_deviation(data: &[f64]) -> f64 {
    variance(data).sqrt()
}

// Monte Carlo simulation function


fn run_simulation(historical_returns: &[f64], bond_returns: &[f64], ss_year: usize, ss_amount: f64) -> (Vec<f64>, Vec<f64>) {
    let mut rng = rng();
    let mut cash: f64 = 250_000.0;
    let mut stock: f64 = INITIAL_PORTFOLIO - cash;
    let mut withdrawals = Vec::with_capacity(YEARS);
    let mut portfolios = Vec::with_capacity(YEARS);

    // Get simulation mode from global AtomicUsize
    let mode = match SIMULATION_MODE.load(Ordering::Relaxed) {
        1 => SimulationMode::Regular4Percent,
        2 => SimulationMode::Fixed35PercentOfPortfolio,
        _ => SimulationMode::GuytonKlinger,
    };

    let mut current_withdrawal = WITHDRAWAL_RATE * INITIAL_PORTFOLIO;

    for year in 0..YEARS {
        let cash_target = 500_000.0 * (1.0 + INFLATION_RATE).powi(year as i32);

        // Grow cash and stock
        let bond_return = bond_returns[rng.random_range(0..bond_returns.len())];
        let stock_return = historical_returns[rng.random_range(0..historical_returns.len())];
        cash *= 1.0 + bond_return;
        stock *= 1.0 + stock_return;

        // If cash exceeds target, move excess to stock
        if cash > cash_target {
            let excess = cash - cash_target;
            stock += excess;
            cash = cash_target;
        }

        let portfolio_value = cash + stock;
        let mut withdrawal;
        match mode {
            SimulationMode::GuytonKlinger => {
                let withdrawal_rate = current_withdrawal / portfolio_value;
                if withdrawal_rate > UPPER_GUARDRAIL {
                    current_withdrawal *= 1.0 - WITHDRAWAL_ADJUSTMENT; // Reduce withdrawal by 10%
                } else if withdrawal_rate < LOWER_GUARDRAIL {
                    current_withdrawal *= 1.0 + WITHDRAWAL_ADJUSTMENT; // Increase withdrawal by 10%
                } else {
                    current_withdrawal *= 1.0 + INFLATION_RATE; // Only apply inflation if no guardrail triggers
                }
                withdrawal = current_withdrawal;
            }
            SimulationMode::Regular4Percent => {
                withdrawal = WITHDRAWAL_RATE * INITIAL_PORTFOLIO * (1.0 + INFLATION_RATE).powi(year as i32);
            }
            SimulationMode::Fixed35PercentOfPortfolio => {
                withdrawal = WITHDRAWAL_RATE * portfolio_value;
            }
        }

        // Social Security adjustment
        if year >= ss_year {
            let ss_adjusted = ss_amount * (1.0 + INFLATION_RATE).powi(year as i32);
            withdrawal = (withdrawal - ss_adjusted).max(0.0);
        }

        // Withdraw from stock first, then cash
        let mut stock_withdrawn = withdrawal.min(stock);
        let mut cash_withdrawn = withdrawal - stock_withdrawn;
        if cash_withdrawn > cash {
            cash_withdrawn = cash;
            stock_withdrawn = withdrawal - cash_withdrawn;
        }

        stock -= stock_withdrawn;
        cash -= cash_withdrawn;

        // If stock is depleted, use cash for remaining withdrawal
        if stock < 0.0 {
            cash += stock; // stock is negative, so this subtracts the deficit from cash
            stock = 0.0;
        }
        if cash < 0.0 {
            cash = 0.0;
        }

        let portfolio_value = cash + stock;
        withdrawals.push(withdrawal);
        portfolios.push(portfolio_value);

        if portfolio_value <= 0.0 {
            withdrawals.resize(YEARS, 0.0);
            portfolios.resize(YEARS, 0.0);
            break;
        }
    }
    (withdrawals, portfolios)
}



fn main() {
    let args: Vec<String> = env::args().collect();
    let mut mode = SimulationMode::GuytonKlinger;
    if args.len() > 1 {
        match args[1].to_lowercase().as_str() {
            "regular" | "4percent" | "4%" => mode = SimulationMode::Regular4Percent,
            "fixed" | "fixed35" | "3.5" | "3.5%" => mode = SimulationMode::Fixed35PercentOfPortfolio,
            "gk" | "guyton" | "guytonklinger" => mode = SimulationMode::GuytonKlinger,
            _ => println!("Unknown mode '{}', defaulting to Guyton-Klinger", args[1]),
        }
    }
    // Set the global simulation mode
    let mode_int = match mode {
        SimulationMode::Regular4Percent => 1,
        SimulationMode::Fixed35PercentOfPortfolio => 2,
        SimulationMode::GuytonKlinger => 0,
    };
    SIMULATION_MODE.store(mode_int, Ordering::Relaxed);

    println!("Simulation mode: {:?}", mode);
    println!(
        "Initial withdrawal rate of {} {}",
        WITHDRAWAL_RATE,
        WITHDRAWAL_RATE * INITIAL_PORTFOLIO
    );

    // Social Security info is now scenario-specific and printed per scenario

    println!("S&P 500 Arithmetic Mean: {:.4}", arithmetic_mean(&SP500_RETURNS));
    println!("S&P 500 Geometric Mean: {:.4}", geometric_mean(&SP500_RETURNS));
    println!("Variance: {:.4}", variance(&SP500_RETURNS));
    println!("S&P 500 Standard Deviation: {:.4}", standard_deviation(&SP500_RETURNS));

    // --- Social Security Scenarios ---
    let scenarios = [
        (2032 - 2026, 2919.0 + 3633.0, "SS: 2919+3633 from 2032"),
        (2040 - 2026, 5210.0 + 4677.0, "SS: 5210+4677 from 2040"),
    ];
    let percentiles = [0.01, 0.02, 0.05, 0.10, 0.25, 0.50, 0.75, 0.90, 0.99];
    let start_year = 2026;
    let csv_years: Vec<usize> = (2030..=2060).step_by(2).collect();
    for (ss_year, ss_amount, label) in scenarios.iter() {
        let mut all_withdrawals: Vec<Vec<f64>> = Vec::with_capacity(SIMULATIONS);
        let mut all_portfolios: Vec<Vec<f64>> = Vec::with_capacity(SIMULATIONS);
        for _ in 0..SIMULATIONS {
            let (withdrawals, portfolios) = run_simulation(&SP500_RETURNS, &BOND_RETURNS, *ss_year, *ss_amount);
            all_withdrawals.push(withdrawals);
            all_portfolios.push(portfolios);
        }
        let year_2045 = 2045 - start_year;
        let mut sim_refs: Vec<(&Vec<f64>, &Vec<f64>)> = all_withdrawals.iter().zip(all_portfolios.iter()).collect();
        sim_refs.sort_by(|a, b| a.1[year_2045].partial_cmp(&b.1[year_2045]).unwrap());
        println!("\n=== Scenario: {} ===", label);
        println!("Number of simulations {}", sim_refs.len());
        println!("percentile\t{}", csv_years.iter().map(|y| y.to_string()).collect::<Vec<_>>().join("\t\t"));
        for &p in &percentiles {
            let idx = (p * SIMULATIONS as f64).round() as usize;
            let idx = idx.min(SIMULATIONS - 1);
            let mut withdrawal_refs = Vec::new();
            if idx > 0 {
                withdrawal_refs.push(sim_refs[idx-1].0);
            }
            withdrawal_refs.push(sim_refs[idx].0);
            if idx + 1 < SIMULATIONS {
                withdrawal_refs.push(sim_refs[idx+1].0);
            }
            let mut row = Vec::new();
            for year in csv_years.iter() {
                let sim_year = year - start_year;
                let inflation_factor = (1.0 + INFLATION_RATE).powi(sim_year as i32);
                let mut vals: Vec<f64> = withdrawal_refs.iter().map(|w| w[sim_year]).collect();
                vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let median = if vals.len() == 3 {
                    vals[1]
                } else if vals.len() == 2 {
                    (vals[0] + vals[1]) / 2.0
                } else {
                    vals[0]
                };
                let mut income = median / inflation_factor;
                if sim_year >= *ss_year {
                    let ss_income = *ss_amount * inflation_factor;
                    income = income + ss_income/inflation_factor;
                }
                row.push(format!("{:.2}", income));
            }
            println!("{:.2}\t{}", p * 100.0, row.join("\t"));
        }
    }
}
