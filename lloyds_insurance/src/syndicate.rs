use crate::{
    Event, ExposureDecision, ModelConfig, PolicyParticipation, Stats, SyndicateStats,
    syndicate_var_exposure::VarExposureManager,
};
use des::{Agent, Response};

/// Simplified Syndicate agent (Phase 1: Basic actuarial pricing only)
pub struct Syndicate {
    syndicate_id: usize,
    config: ModelConfig,
    capital: f64,
    policies: Vec<PolicyParticipation>,
    loss_history: Vec<f64>, // Tracks CLAIM AMOUNTS (when claims occur)
    premium_history: Vec<f64>,
    stats: SyndicateStats,

    // Annual tracking for dividend calculation and reporting
    annual_premiums: f64,
    annual_claims: f64,
    annual_policies_written: usize,
    annual_claims_count: usize,

    // Underwriting markup: exponentially weighted moving average of market conditions
    // m_t captures competitive pressure based on loss experience
    markup_m_t: f64,

    // Loss ratio history for lagged markup update (fixes cohort mismatch)
    // Use prior year's loss ratio for responsive pricing
    // (1-year lag allows timely adjustment while providing stable signal)
    prior_year_loss_ratio: Option<f64>, // Year N-1

    // Dynamic industry statistics (updated annually from MarketStatisticsCollector)
    // These replace the hardcoded config values for actuarial pricing
    industry_lambda_t: f64, // Industry-wide claim frequency (claims per policy)
    industry_mu_t: f64,     // Industry-wide average claim cost
    years_elapsed: usize,   // Track years for warmup period

    // VaR-based exposure management (optional - enabled based on config)
    var_exposure_manager: Option<VarExposureManager>,
}

impl Syndicate {
    pub fn new(syndicate_id: usize, config: ModelConfig) -> Self {
        let initial_capital = config.initial_capital;
        // Initialize with config defaults until first market stats are available
        // NOTE: Config values are per-risk, but we interpret them as per-participation initially
        // This means we assume default lead line size for initialization
        let industry_lambda_t = config.yearly_claim_frequency; // Claims per participation ≈ claims per risk
        let industry_mu_t = config.gamma_mean * config.default_lead_line_size; // Avg claim for 50% line

        // Initialize VaR exposure manager if enabled (var_exceedance_prob > 0)
        let var_exposure_manager = if config.var_exceedance_prob > 0.0 {
            // Use syndicate_id as seed for deterministic behavior per syndicate
            Some(VarExposureManager::new(
                config.clone(),
                initial_capital,
                syndicate_id as u64 + 1000,
            ))
        } else {
            None
        };

        Self {
            syndicate_id,
            config,
            capital: initial_capital,
            policies: Vec::new(),
            loss_history: Vec::new(),
            premium_history: Vec::new(),
            stats: SyndicateStats::new(syndicate_id, initial_capital),
            annual_premiums: 0.0,
            annual_claims: 0.0,
            annual_policies_written: 0,
            annual_claims_count: 0,
            // Start at actuarially fair pricing (no initial markup)
            // Markup will adjust based on observed loss experience
            markup_m_t: 0.0,
            prior_year_loss_ratio: None, // No prior experience yet
            industry_lambda_t,
            industry_mu_t,
            years_elapsed: 0,
            var_exposure_manager,
        }
    }

    fn calculate_actuarial_price(
        &self,
        _risk_id: usize,
        industry_avg_loss_per_participation: f64,
    ) -> f64 {
        // Simplified actuarial pricing: P̃_t = z·X̄_t + (1-z)·λ'_t·μ'_t
        //
        // ALL values are interpreted as PER-PARTICIPATION:
        // - industry_avg_loss_per_participation = industry_lambda_t × industry_mu_t
        // - Where industry_mu_t is average claim amount received (line-share adjusted)
        // - And industry_lambda_t is claim frequency per participation
        //
        // This matches how syndicates calculate their own experience (based on participations)

        let z = self.config.internal_experience_weight;
        let claim_freq = self.config.yearly_claim_frequency;

        // Syndicate's own experience (expected loss per participation based on own data)
        let syndicate_expected_loss = if !self.loss_history.is_empty() {
            // Exponentially weighted moving average of CLAIM AMOUNTS (line-share adjusted)
            let weight = self.config.loss_recency_weight;
            let mut weighted_sum = 0.0;
            let mut total_weight = 0.0;
            for (i, loss) in self.loss_history.iter().rev().enumerate() {
                let w = (1.0 - weight).powi(i as i32);
                weighted_sum += loss * w;
                total_weight += w;
            }
            let avg_claim_amount = weighted_sum / total_weight;
            // Convert to expected loss per participation: E[loss] = P(claim) × E[amount | claim]
            avg_claim_amount * claim_freq
        } else {
            // No history yet - use industry average (already per-participation)
            industry_avg_loss_per_participation
        };

        // Industry expected loss per participation (use directly)
        let industry_expected_loss = industry_avg_loss_per_participation;

        // Combine syndicate and industry experience
        let base_price = z * syndicate_expected_loss + (1.0 - z) * industry_expected_loss;

        // Add volatility loading
        let volatility_loading = self.config.volatility_weight * base_price;

        base_price + volatility_loading
    }

    fn apply_underwriting_markup(&self, actuarial_price: f64) -> f64 {
        // Apply underwriting markup: P_t = P_at · e^(m_t)
        // Where m_t is an EWMA capturing competitive pressure
        // - m_t > 0: recent losses high → increase premium
        // - m_t = 0: balanced → no adjustment
        // - m_t < 0: recent profits high → decrease premium (competitive pressure)
        actuarial_price * self.markup_m_t.exp()
    }

    /// Premium-based exposure management (Scenario 1)
    ///
    /// Simple exposure management using premium-to-capital ratio.
    /// Returns ExposureDecision based on whether adding the proposed premium
    /// would exceed the premium_reserve_ratio threshold.
    fn check_premium_exposure(&self, proposed_premium: f64) -> ExposureDecision {
        if self.capital <= 0.0 {
            // No capital → reject
            return ExposureDecision::Reject;
        }

        // Calculate premium-to-capital ratio after accepting this quote
        let proposed_total_premium = self.annual_premiums + proposed_premium;
        let premium_to_capital_ratio = proposed_total_premium / self.capital;

        // Check against threshold
        let threshold = self.config.premium_reserve_ratio;

        if premium_to_capital_ratio <= threshold {
            // Within limits
            ExposureDecision::Accept
        } else {
            // Exceeds threshold - either reject or scale premium up
            // Scaling premium up makes quote less attractive, reducing our participation
            let excess_ratio = premium_to_capital_ratio / threshold;
            if excess_ratio > 2.0 {
                // Far over threshold → reject
                ExposureDecision::Reject
            } else {
                // Moderately over → scale premium up
                ExposureDecision::ScalePremium(excess_ratio)
            }
        }
    }

    fn handle_lead_quote_request(
        &mut self,
        risk_id: usize,
        _peril_region: usize,
        _risk_limit: f64,
        current_t: usize,
    ) -> Vec<(usize, Event)> {
        // Use dynamic industry statistics (updated annually from market data)
        let industry_avg_loss = self.industry_mu_t * self.industry_lambda_t;
        let line_size = self.config.default_lead_line_size;

        // Calculate actuarial price and apply underwriting markup
        let actuarial_price = self.calculate_actuarial_price(risk_id, industry_avg_loss);
        let mut price = self.apply_underwriting_markup(actuarial_price);

        // Exposure management: Use VaR EM if enabled (Scenario 3), otherwise use Premium EM (Scenario 1)
        if let Some(ref mut var_em) = self.var_exposure_manager {
            // VaR-based exposure management (Scenario 3)
            let proposed_exposure = line_size * _risk_limit;
            match var_em.evaluate_quote(_peril_region, proposed_exposure) {
                ExposureDecision::Accept => {
                    // Proceed with quote
                }
                ExposureDecision::Reject => {
                    // Decline to quote
                    return Vec::new();
                }
                ExposureDecision::ScalePremium(factor) => {
                    // Scale premium up to compensate for risk
                    price *= factor;
                }
            }
        } else {
            // Premium-based exposure management (Scenario 1)
            // Only applies when VaR EM is not enabled
            let proposed_premium = price;
            match self.check_premium_exposure(proposed_premium) {
                ExposureDecision::Accept => {
                    // Proceed with quote
                }
                ExposureDecision::Reject => {
                    // Decline to quote - premium-to-capital ratio too high
                    return Vec::new();
                }
                ExposureDecision::ScalePremium(factor) => {
                    // Scale premium up to reduce attractiveness
                    price *= factor;
                }
            }
        }

        vec![(
            current_t,
            Event::LeadQuoteOffered {
                risk_id,
                syndicate_id: self.syndicate_id,
                price,
                line_size,
            },
        )]
    }

    fn handle_lead_accepted(&mut self, risk_id: usize, peril_region: usize, risk_limit: f64) {
        // Record premium - must match what we quoted
        let industry_avg_loss = self.industry_mu_t * self.industry_lambda_t;
        let line_size = self.config.default_lead_line_size;

        // Calculate actuarial price and apply underwriting markup (must match quote)
        let actuarial_price = self.calculate_actuarial_price(risk_id, industry_avg_loss);
        let price = self.apply_underwriting_markup(actuarial_price);

        self.capital += price;
        self.premium_history.push(price);
        self.annual_premiums += price;
        self.annual_policies_written += 1;

        // Track exposure by peril region
        let exposure = line_size * risk_limit;
        *self
            .stats
            .exposure_by_peril_region
            .entry(peril_region)
            .or_insert(0.0) += exposure;

        // Record exposure in VaR manager and update capital
        if let Some(ref mut var_em) = self.var_exposure_manager {
            var_em.record_exposure(peril_region, exposure);
            var_em.update_capital(self.capital);
        }

        let participation = PolicyParticipation {
            risk_id,
            line_size,
            premium_collected: price,
            is_lead: true,
        };
        self.policies.push(participation);

        self.stats.num_policies += 1;
        self.stats.total_premium_written += price;
        self.stats.total_premiums_collected += price;
        self.stats.total_line_size += line_size;
    }

    fn handle_follow_quote_request(
        &mut self,
        risk_id: usize,
        lead_price: f64,
        peril_region: usize,
        risk_limit: f64,
        current_t: usize,
    ) -> Vec<(usize, Event)> {
        // Calculate what we think the price should be (our independent assessment)
        let baseline_line_size = self.config.default_follow_line_size;
        let industry_avg_loss = self.industry_mu_t * self.industry_lambda_t;
        let actuarial_price = self.calculate_actuarial_price(risk_id, industry_avg_loss);
        let my_price = self.apply_underwriting_markup(actuarial_price);

        // Calculate pricing strength: my_price / lead_price
        // - pricing_strength > 1.0: Lead price is higher than we think → favorable, offer more
        // - pricing_strength < 1.0: Lead price is lower than we think → unfavorable, offer less
        // - pricing_strength < 0.5: Lead price is too low → decline to quote
        let pricing_strength = if lead_price > 0.0 {
            my_price / lead_price
        } else {
            0.0
        };

        // Adjust line size based on pricing strength
        let line_size = if pricing_strength < 0.5 {
            // Lead price is very unfavorable (more than 2x what we think it should be)
            // Decline to quote
            return Vec::new();
        } else if pricing_strength > 1.0 {
            // Lead price is favorable (lower than what we would charge)
            // Offer more, but cap at 2x baseline
            let scale_factor = pricing_strength.min(2.0);
            baseline_line_size * scale_factor
        } else {
            // Lead price is somewhat unfavorable but acceptable
            // Scale down proportionally
            baseline_line_size * pricing_strength
        };

        // VaR exposure management check
        if let Some(ref mut var_em) = self.var_exposure_manager {
            let proposed_exposure = line_size * risk_limit;
            match var_em.evaluate_quote(peril_region, proposed_exposure) {
                ExposureDecision::Accept => {
                    // Proceed with quote
                }
                ExposureDecision::Reject => {
                    // Decline to quote
                    return Vec::new();
                }
                ExposureDecision::ScalePremium(_factor) => {
                    // For followers, we can't scale premium (we accept lead's price)
                    // Instead, reduce line size to manage exposure
                    // Decline the quote if VaR suggests scaling
                    return Vec::new();
                }
            }
        }

        vec![(
            current_t,
            Event::FollowQuoteOffered {
                risk_id,
                syndicate_id: self.syndicate_id,
                line_size,
            },
        )]
    }

    fn handle_follow_accepted(
        &mut self,
        risk_id: usize,
        line_size: f64,
        peril_region: usize,
        risk_limit: f64,
    ) {
        // Calculate premium for our follow share
        let industry_avg_loss = self.industry_mu_t * self.industry_lambda_t;

        // For followers, we use the same pricing logic but with the follow line size
        // (which is passed in, not the default)
        let full_risk_actuarial = self.calculate_actuarial_price(risk_id, industry_avg_loss);
        let full_risk_price = self.apply_underwriting_markup(full_risk_actuarial);

        // Adjust for the actual line size allocated (may be less than requested)
        let price = (full_risk_price / self.config.default_lead_line_size) * line_size;

        self.capital += price;
        self.premium_history.push(price);
        self.annual_premiums += price;
        self.annual_policies_written += 1;

        // Track exposure by peril region
        let exposure = line_size * risk_limit;
        *self
            .stats
            .exposure_by_peril_region
            .entry(peril_region)
            .or_insert(0.0) += exposure;

        // Record exposure in VaR manager and update capital
        if let Some(ref mut var_em) = self.var_exposure_manager {
            var_em.record_exposure(peril_region, exposure);
            var_em.update_capital(self.capital);
        }

        let participation = PolicyParticipation {
            risk_id,
            line_size,
            premium_collected: price,
            is_lead: false,
        };
        self.policies.push(participation);

        self.stats.num_policies += 1;
        self.stats.total_premium_written += price;
        self.stats.total_premiums_collected += price;
        self.stats.total_line_size += line_size;
    }

    fn handle_claim(&mut self, _risk_id: usize, amount: f64) -> Vec<(usize, Event)> {
        self.capital -= amount;
        self.loss_history.push(amount);
        self.annual_claims += amount;
        self.annual_claims_count += 1;

        // Update VaR manager capital if enabled
        if let Some(ref mut var_em) = self.var_exposure_manager {
            var_em.update_capital(self.capital);
        }

        self.stats.total_claims_paid += amount;
        self.stats.num_claims += 1;

        // Check for insolvency
        if self.capital <= 0.0 {
            self.stats.is_insolvent = true;
            vec![(
                0,
                Event::SyndicateBankrupted {
                    syndicate_id: self.syndicate_id,
                },
            )]
        } else {
            Vec::new()
        }
    }

    fn handle_year_end(&mut self) {
        // Track years for warmup period
        self.years_elapsed += 1;

        // Update underwriting markup BEFORE checking insolvency
        // Even insolvent syndicates update their market view (though they won't quote)
        self.update_underwriting_markup();

        // Insolvent syndicates don't pay dividends and reset annual counters
        if self.stats.is_insolvent {
            self.annual_premiums = 0.0;
            self.annual_claims = 0.0;
            self.annual_policies_written = 0;
            self.annual_claims_count = 0;
            return;
        }

        // Calculate annual profit: Pr_t = premiums - claims
        let annual_profit = self.annual_premiums - self.annual_claims;

        // Pay dividend only if there's positive profit: D = γ · Pr_t
        // Also check that we have sufficient capital to avoid causing insolvency via dividends
        if annual_profit > 0.0 {
            let dividend = self.config.profit_fraction * annual_profit;
            if self.capital >= dividend {
                self.capital -= dividend;
                self.stats.total_dividends_paid += dividend;

                // Update VaR manager capital if enabled
                if let Some(ref mut var_em) = self.var_exposure_manager {
                    var_em.update_capital(self.capital);
                }
            }
        }

        // Reset annual counters
        self.annual_premiums = 0.0;
        self.annual_claims = 0.0;
        self.annual_policies_written = 0;
        self.annual_claims_count = 0;
    }

    fn update_underwriting_markup(&mut self) {
        // Update m_t using EWMA: m_t = β · m_{t-1} + (1-β) · signal_t
        // where signal_t = log(loss_ratio_t)
        //
        // This captures competitive pressure:
        // - High loss ratios (>1) → positive signal → m_t increases → higher premiums
        // - Low loss ratios (<1) → negative signal → m_t decreases → lower premiums
        // - Balanced loss ratios (≈1) → signal ≈ 0 → m_t decays toward 0
        //
        // NOTE: Uses prior year's loss ratio to allow for responsive pricing.
        // Year 0 may show slightly low loss ratios due to calendar-year accounting
        // (policies written late in year have claims in year 1), but this is a minor
        // initialization artifact that resolves quickly.

        let current_year_loss_ratio = if self.annual_premiums > 0.0 {
            Some(self.annual_claims / self.annual_premiums)
        } else {
            None
        };

        // Update markup using prior year's loss ratio (if available)
        if let Some(prior_loss_ratio) = self.prior_year_loss_ratio {
            let signal = prior_loss_ratio.ln(); // log(loss_ratio)
            let beta = self.config.underwriter_recency_weight;

            // EWMA update
            self.markup_m_t = beta * self.markup_m_t + (1.0 - beta) * signal;
        }
        // Year 0: No prior data, markup stays at initial value (0.0)

        // Shift history: current → prior
        self.prior_year_loss_ratio = current_year_loss_ratio;
    }

    fn update_stats(&mut self) {
        self.stats.capital = self.capital;
        self.stats.update_loss_ratio();
        self.stats.update_profit();
        self.stats.markup_m_t = self.markup_m_t;

        // Update uniform_deviation from VaR manager if enabled
        if let Some(ref var_em) = self.var_exposure_manager {
            self.stats.uniform_deviation = var_em.uniform_deviation();
        } else {
            self.stats.uniform_deviation = 0.0;
        }
    }
}

impl Agent<Event, Stats> for Syndicate {
    fn act(&mut self, current_t: usize, data: &Event) -> Response<Event, Stats> {
        // EXCEPTION: Handle Year events even if insolvent (for market statistics reporting)
        //
        // Rationale: Insolvent syndicates must still report their capital state to the
        // MarketStatisticsCollector for accurate time series data. This is the ONLY event
        // that bypasses the insolvency check, as it represents regulatory reporting
        // (which continues even after insolvency) rather than active market participation.
        //
        // The handle_year_end() method includes its own insolvency check to ensure
        // insolvent syndicates don't pay dividends.
        if matches!(data, Event::Year) {
            // Capture annual metrics BEFORE handle_year_end() resets them
            let annual_premiums = self.annual_premiums;
            let annual_claims = self.annual_claims;
            let annual_policies_written = self.annual_policies_written;
            let annual_claims_count = self.annual_claims_count;

            self.handle_year_end();
            self.update_stats();

            // Calculate uniform_deviation from stats
            let uniform_deviation = self.stats.uniform_deviation;

            // Report capital to market statistics collector
            return Response::events(vec![(
                current_t,
                Event::SyndicateCapitalReported {
                    syndicate_id: self.syndicate_id,
                    capital: self.capital,
                    annual_premiums,
                    annual_claims,
                    num_policies: annual_policies_written,
                    num_claims: annual_claims_count,
                    markup_m_t: self.markup_m_t,
                    uniform_deviation,
                },
            )]);
        }

        if self.stats.is_insolvent {
            return Response::new();
        }

        match data {
            Event::LeadQuoteRequested {
                risk_id,
                syndicate_id,
                peril_region,
                risk_limit,
            } if *syndicate_id == self.syndicate_id => Response::events(
                self.handle_lead_quote_request(*risk_id, *peril_region, *risk_limit, current_t),
            ),
            Event::LeadQuoteAccepted {
                risk_id,
                syndicate_id,
                peril_region,
                risk_limit,
            } if *syndicate_id == self.syndicate_id => {
                self.handle_lead_accepted(*risk_id, *peril_region, *risk_limit);
                Response::new()
            }
            Event::FollowQuoteRequested {
                risk_id,
                syndicate_id,
                lead_price,
                peril_region,
                risk_limit,
            } if *syndicate_id == self.syndicate_id => {
                Response::events(self.handle_follow_quote_request(
                    *risk_id,
                    *lead_price,
                    *peril_region,
                    *risk_limit,
                    current_t,
                ))
            }
            Event::FollowQuoteAccepted {
                risk_id,
                syndicate_id,
                line_size,
                peril_region,
                risk_limit,
            } if *syndicate_id == self.syndicate_id => {
                self.handle_follow_accepted(*risk_id, *line_size, *peril_region, *risk_limit);
                Response::new()
            }
            Event::ClaimReceived {
                risk_id,
                syndicate_id,
                amount,
            } if *syndicate_id == self.syndicate_id => {
                Response::events(self.handle_claim(*risk_id, *amount))
            }
            Event::Month => {
                self.update_stats();
                Response::new()
            }
            Event::IndustryLossStatsReported {
                avg_claim_frequency,
                avg_claim_cost,
            } => {
                // Update our view of industry-wide loss statistics using EWMA to smooth noise
                // Use a warmup period (first 3 years) to avoid early random variation causing mispricing

                // EWMA weight: give more weight to historical values initially to avoid
                // early year random variation causing systematic mispricing
                let alpha = if self.years_elapsed == 0 {
                    0.1 // Year 0: use only 10% of new data (small sample, high variance)
                } else if self.years_elapsed < 5 {
                    0.2 // Years 1-4: use 20% of new data, 80% of historical
                } else {
                    0.4 // Years 5+: use 40% of new data (more responsive to market changes)
                };

                // Update with exponential smoothing
                self.industry_lambda_t =
                    alpha * avg_claim_frequency + (1.0 - alpha) * self.industry_lambda_t;
                self.industry_mu_t = alpha * avg_claim_cost + (1.0 - alpha) * self.industry_mu_t;

                Response::new()
            }
            _ => Response::new(),
        }
    }

    fn stats(&self) -> Stats {
        Stats::SyndicateStats(self.stats.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_actuarial_price_calculation() {
        let config = ModelConfig::default();
        let syndicate = Syndicate::new(0, config.clone());

        // With no history, should use industry average (per-participation)
        // Syndicate is initialized with per-participation industry stats:
        // industry_mu_t = gamma_mean × default_lead_line_size = $3M × 0.5 = $1.5M
        // industry_lambda_t = yearly_claim_frequency = 0.1
        let industry_avg_per_participation = syndicate.industry_mu_t * syndicate.industry_lambda_t;
        let base_price = industry_avg_per_participation; // $150k
        let volatility_loading = config.volatility_weight * base_price;
        let expected_price = base_price + volatility_loading;

        let price = syndicate.calculate_actuarial_price(1, industry_avg_per_participation);

        // Price should equal base price plus volatility loading
        // With volatility_weight=0.2: $150k base + $30k loading = $180k
        assert!(
            (price - expected_price).abs() < 1.0,
            "Expected ${:.0} (base ${:.0} + loading ${:.0}), got ${:.0}",
            expected_price,
            base_price,
            volatility_loading,
            price
        );
    }

    #[test]
    fn test_syndicate_insolvency() {
        let config = ModelConfig::default();
        let mut syndicate = Syndicate::new(0, config);

        // Large claim that exceeds capital
        let events = syndicate.handle_claim(1, 20_000_000.0);

        assert!(syndicate.stats.is_insolvent);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0].1, Event::SyndicateBankrupted { .. }));
    }

    #[test]
    fn test_syndicate_collects_premium() {
        let config = ModelConfig::default();
        let mut syndicate = Syndicate::new(0, config.clone());
        let initial_capital = syndicate.capital;

        syndicate.handle_lead_accepted(1, 0, 10_000_000.0);

        assert!(syndicate.capital > initial_capital);
        assert_eq!(syndicate.stats.num_policies, 1);
    }

    #[test]
    fn test_syndicate_responds_to_follow_quote_request() {
        // RED: This test should FAIL because syndicates don't handle follow quotes yet
        let config = ModelConfig::default();
        let mut syndicate = Syndicate::new(0, config.clone());

        let resp = syndicate.act(
            0,
            &Event::FollowQuoteRequested {
                risk_id: 1,
                syndicate_id: 0,
                lead_price: 150_000.0,
                peril_region: 0,
                risk_limit: 10_000_000.0,
            },
        );

        // Should generate a FollowQuoteOffered event
        assert_eq!(resp.events.len(), 1);
        assert!(
            matches!(
                resp.events[0].1,
                Event::FollowQuoteOffered {
                    risk_id: 1,
                    syndicate_id: 0,
                    ..
                }
            ),
            "Syndicate should offer a follow quote when requested"
        );
    }

    #[test]
    fn test_syndicate_accepts_follow_quote() {
        // RED: This test should FAIL because syndicates don't handle follow acceptance yet
        let config = ModelConfig::default();
        let mut syndicate = Syndicate::new(0, config.clone());
        let initial_capital = syndicate.capital;

        syndicate.act(
            0,
            &Event::FollowQuoteAccepted {
                risk_id: 1,
                syndicate_id: 0,
                line_size: 0.1,
                peril_region: 0,
                risk_limit: 10_000_000.0,
            },
        );

        // Should collect premium and record policy
        assert!(
            syndicate.capital > initial_capital,
            "Should collect premium"
        );
        assert_eq!(syndicate.stats.num_policies, 1, "Should record policy");
    }

    #[test]
    fn test_dividend_payment_on_profitable_year() {
        let config = ModelConfig::default();
        let mut syndicate = Syndicate::new(0, config.clone());
        let initial_capital = syndicate.capital;

        // Simulate a profitable year: collect premiums and pay fewer claims
        syndicate.annual_premiums = 1_000_000.0;
        syndicate.annual_claims = 600_000.0;

        // Call year-end handler
        syndicate.handle_year_end();

        // Annual profit = 1M - 600k = 400k
        // Dividend = 0.4 * 400k = 160k
        let expected_dividend = 160_000.0;
        let expected_capital = initial_capital - expected_dividend;

        assert_eq!(
            syndicate.stats.total_dividends_paid, expected_dividend,
            "Dividend should be 40% of annual profit"
        );
        assert_eq!(
            syndicate.capital, expected_capital,
            "Capital should be reduced by dividend"
        );
        assert_eq!(
            syndicate.annual_premiums, 0.0,
            "Annual premiums should reset"
        );
        assert_eq!(syndicate.annual_claims, 0.0, "Annual claims should reset");
    }

    #[test]
    fn test_no_dividend_on_loss() {
        let config = ModelConfig::default();
        let mut syndicate = Syndicate::new(0, config.clone());
        let initial_capital = syndicate.capital;

        // Simulate a loss year: claims exceed premiums
        syndicate.annual_premiums = 600_000.0;
        syndicate.annual_claims = 1_000_000.0;

        // Call year-end handler
        syndicate.handle_year_end();

        // No dividend should be paid when there's a loss
        assert_eq!(
            syndicate.stats.total_dividends_paid, 0.0,
            "No dividend should be paid on loss"
        );
        assert_eq!(
            syndicate.capital, initial_capital,
            "Capital should not change from dividend (only from claims)"
        );
    }

    #[test]
    fn test_dividend_accumulates_over_years() {
        let config = ModelConfig::default();
        let mut syndicate = Syndicate::new(0, config.clone());

        // Year 1: profit of 400k
        syndicate.annual_premiums = 1_000_000.0;
        syndicate.annual_claims = 600_000.0;
        syndicate.handle_year_end();

        let year1_dividend = 160_000.0; // 0.4 * 400k
        assert_eq!(syndicate.stats.total_dividends_paid, year1_dividend);

        // Year 2: profit of 200k
        syndicate.annual_premiums = 800_000.0;
        syndicate.annual_claims = 600_000.0;
        syndicate.handle_year_end();

        let year2_dividend = 80_000.0; // 0.4 * 200k
        let total_dividends = year1_dividend + year2_dividend;

        assert_eq!(
            syndicate.stats.total_dividends_paid, total_dividends,
            "Dividends should accumulate over years"
        );
    }

    #[test]
    fn test_year_event_triggers_dividend() {
        let config = ModelConfig::default();
        let mut syndicate = Syndicate::new(0, config.clone());
        let initial_capital = syndicate.capital;

        // Simulate activity
        syndicate.annual_premiums = 500_000.0;
        syndicate.annual_claims = 300_000.0;

        // Send Year event
        syndicate.act(365, &Event::Year);

        // Should have paid dividend
        let expected_dividend = 0.4 * (500_000.0 - 300_000.0); // 0.4 * 200k = 80k
        assert_eq!(syndicate.stats.total_dividends_paid, expected_dividend);
        assert_eq!(syndicate.capital, initial_capital - expected_dividend);
    }

    #[test]
    fn test_no_dividend_when_insufficient_capital() {
        let config = ModelConfig::default();
        let mut syndicate = Syndicate::new(0, config.clone());

        // Set low capital
        syndicate.capital = 50_000.0;

        // High profit would normally trigger large dividend
        syndicate.annual_premiums = 1_000_000.0;
        syndicate.annual_claims = 800_000.0;
        // Annual profit = 200k, dividend would be 80k, but capital is only 50k

        syndicate.handle_year_end();

        // Should NOT pay dividend because capital < dividend
        assert_eq!(
            syndicate.stats.total_dividends_paid, 0.0,
            "Should not pay dividend when capital insufficient"
        );
        assert_eq!(syndicate.capital, 50_000.0, "Capital should be unchanged");
    }

    #[test]
    fn test_insolvent_syndicate_no_dividend() {
        let config = ModelConfig::default();
        let mut syndicate = Syndicate::new(0, config.clone());

        // Mark as insolvent
        syndicate.stats.is_insolvent = true;
        syndicate.capital = -100_000.0;

        // Would normally have profit
        syndicate.annual_premiums = 500_000.0;
        syndicate.annual_claims = 300_000.0;

        syndicate.handle_year_end();

        // Should NOT pay dividend when insolvent
        assert_eq!(
            syndicate.stats.total_dividends_paid, 0.0,
            "Insolvent syndicates should not pay dividends"
        );
        assert_eq!(
            syndicate.annual_premiums, 0.0,
            "Annual counters should be reset"
        );
    }

    #[test]
    fn test_quoted_price_matches_accepted_price() {
        // RED: This test should fail because handle_lead_accepted recalculates price
        let config = ModelConfig::default();
        let mut syndicate = Syndicate::new(0, config.clone());

        // Step 1: Quote a price
        let quote_events = syndicate.handle_lead_quote_request(1, 0, 10_000_000.0, 0);
        let quoted_price = match &quote_events[0].1 {
            Event::LeadQuoteOffered { price, .. } => *price,
            _ => panic!("Expected LeadQuoteOffered event"),
        };

        // Step 2: Accept that quote
        let initial_capital = syndicate.capital;
        syndicate.handle_lead_accepted(1, 0, 10_000_000.0);

        // Step 3: Verify the premium collected matches the quoted price
        let premium_collected = syndicate.capital - initial_capital;
        assert!(
            (premium_collected - quoted_price).abs() < 0.01,
            "Premium collected (${:.2}) should match quoted price (${:.2}) within $0.01",
            premium_collected,
            quoted_price
        );

        // Also verify stats match (with floating-point tolerance)
        assert!(
            (syndicate.stats.total_premiums_collected - quoted_price).abs() < 0.01,
            "Stats premium (${:.2}) should match quoted price (${:.2}) within $0.01",
            syndicate.stats.total_premiums_collected,
            quoted_price
        );
    }

    #[test]
    fn test_underwriting_markup_increases_after_losses() {
        let config = ModelConfig::default();
        let mut syndicate = Syndicate::new(0, config.clone());

        // Initial markup is 0.0 (fair pricing)
        syndicate.markup_m_t = 0.0;

        // Simulate a high-loss year: loss_ratio = 2.0
        syndicate.annual_premiums = 1_000_000.0;
        syndicate.annual_claims = 2_000_000.0;

        // Manually set prior year to trigger update (normally this comes from history)
        syndicate.prior_year_loss_ratio = Some(2.0);

        // Update markup at year-end
        syndicate.update_underwriting_markup();

        // markup should be positive: m_t = 0.2 * 0 + 0.8 * ln(2.0) ≈ 0.554
        assert!(
            syndicate.markup_m_t > 0.0,
            "Markup should increase after high losses"
        );
        assert!(
            syndicate.markup_m_t > 0.5 && syndicate.markup_m_t < 0.6,
            "Markup should be around 0.554, got {}",
            syndicate.markup_m_t
        );
    }

    #[test]
    fn test_underwriting_markup_decreases_after_profits() {
        let config = ModelConfig::default();
        let mut syndicate = Syndicate::new(0, config.clone());

        // Start with some positive markup
        syndicate.markup_m_t = 0.5;

        // Manually set prior year to trigger update (normally this comes from history)
        syndicate.prior_year_loss_ratio = Some(0.5);

        // Simulate a profitable year: loss_ratio = 0.5
        syndicate.annual_premiums = 1_000_000.0;
        syndicate.annual_claims = 500_000.0;

        // Update markup
        syndicate.update_underwriting_markup();

        // markup should be less than before: m_t = 0.2 * 0.5 + 0.8 * ln(0.5) ≈ -0.454
        assert!(
            syndicate.markup_m_t < 0.0,
            "Markup should decrease after low losses (profitable period)"
        );
    }

    #[test]
    fn test_underwriting_markup_affects_premium() {
        let config = ModelConfig::default();
        let mut syndicate = Syndicate::new(0, config.clone());
        let industry_avg_loss = config.gamma_mean * config.yearly_claim_frequency;

        // Set markup to 0 for baseline testing
        syndicate.markup_m_t = 0.0;

        // Calculate baseline price with no markup
        let baseline_price = syndicate.calculate_actuarial_price(1, industry_avg_loss);
        let baseline_final = syndicate.apply_underwriting_markup(baseline_price);
        assert_eq!(
            baseline_final, baseline_price,
            "With m_t=0, markup should be 1.0"
        );

        // Set positive markup (simulate post-catastrophe environment)
        syndicate.markup_m_t = 0.5;
        let high_price = syndicate.apply_underwriting_markup(baseline_price);
        assert!(
            high_price > baseline_price,
            "Positive markup should increase price"
        );
        assert!(
            (high_price / baseline_price - 1.0).abs() < 0.01 || high_price / baseline_price > 1.6,
            "e^0.5 ≈ 1.649, so price should be ~65% higher"
        );

        // Set negative markup (simulate very profitable period)
        syndicate.markup_m_t = -0.5;
        let low_price = syndicate.apply_underwriting_markup(baseline_price);
        assert!(
            low_price < baseline_price,
            "Negative markup should decrease price"
        );
        assert!(
            (low_price / baseline_price).abs() < 1.0,
            "e^-0.5 ≈ 0.606, so price should be ~40% lower"
        );
    }

    #[test]
    fn test_pricing_strength_adjusts_follow_line_size() {
        let config = ModelConfig::default();
        let mut syndicate = Syndicate::new(0, config.clone());
        let baseline_line_size = config.default_follow_line_size;

        // Calculate what the syndicate thinks is a fair price
        let industry_avg_loss = syndicate.industry_mu_t * syndicate.industry_lambda_t;
        let actuarial_price = syndicate.calculate_actuarial_price(1, industry_avg_loss);
        let fair_price = syndicate.apply_underwriting_markup(actuarial_price);

        // Case 1: Lead price is favorable (lower than our assessment)
        // pricing_strength = our_price / lead_price > 1.0 → offer more
        let favorable_lead_price = fair_price * 0.5; // Half of what we'd charge
        let resp =
            syndicate.handle_follow_quote_request(1, favorable_lead_price, 0, 10_000_000.0, 0);
        assert!(
            !resp.is_empty(),
            "Should quote when lead price is favorable"
        );
        if let Event::FollowQuoteOffered { line_size, .. } = &resp[0].1 {
            assert!(
                *line_size > baseline_line_size,
                "Should offer more than baseline when lead price is favorable: {} vs {}",
                line_size,
                baseline_line_size
            );
        }

        // Case 2: Lead price is at our assessment (pricing_strength ≈ 1.0)
        let fair_lead_price = fair_price;
        let resp = syndicate.handle_follow_quote_request(1, fair_lead_price, 0, 10_000_000.0, 0);
        assert!(!resp.is_empty(), "Should quote when lead price is fair");
        if let Event::FollowQuoteOffered { line_size, .. } = &resp[0].1 {
            assert!(
                (*line_size - baseline_line_size).abs() < 0.01,
                "Should offer baseline when lead price is fair: {} vs {}",
                line_size,
                baseline_line_size
            );
        }

        // Case 3: Lead price is somewhat unfavorable (pricing_strength between 0.5 and 1.0)
        // Should scale down but still quote
        let unfavorable_lead_price = fair_price * 1.5; // 50% higher than we'd charge
        let resp =
            syndicate.handle_follow_quote_request(1, unfavorable_lead_price, 0, 10_000_000.0, 0);
        assert!(
            !resp.is_empty(),
            "Should still quote when lead price is moderately unfavorable"
        );
        if let Event::FollowQuoteOffered { line_size, .. } = &resp[0].1 {
            assert!(
                *line_size < baseline_line_size,
                "Should offer less than baseline when lead price is unfavorable: {} vs {}",
                line_size,
                baseline_line_size
            );
        }

        // Case 4: Lead price is very unfavorable (pricing_strength < 0.5)
        // Should decline to quote
        let very_unfavorable_lead_price = fair_price * 3.0; // 3x what we'd charge
        let resp = syndicate.handle_follow_quote_request(
            1,
            very_unfavorable_lead_price,
            0,
            10_000_000.0,
            0,
        );
        assert!(
            resp.is_empty(),
            "Should decline to quote when lead price is very unfavorable (pricing_strength < 0.5)"
        );
    }

    #[test]
    fn test_premium_based_exposure_management() {
        // Test Scenario 1: Premium-based EM (when VaR EM is disabled)
        let config = ModelConfig {
            var_exceedance_prob: 0.0,   // Disable VaR EM → use Premium EM
            premium_reserve_ratio: 0.5, // Max 50% premium-to-capital ratio
            ..Default::default()
        };
        let mut syndicate = Syndicate::new(0, config.clone());

        // Set capital to a known value
        syndicate.capital = 10_000_000.0;
        syndicate.annual_premiums = 0.0;

        // Case 1: Small premium (well within limits)
        // premium_to_capital = 100k / 10M = 0.01 < 0.5 → Accept
        let small_premium = 100_000.0;
        let decision = syndicate.check_premium_exposure(small_premium);
        assert_eq!(
            decision,
            ExposureDecision::Accept,
            "Small premium should be accepted"
        );

        // Case 2: Moderate premium (approaching threshold)
        // premium_to_capital = 4M / 10M = 0.4 < 0.5 → Accept
        syndicate.annual_premiums = 0.0;
        let moderate_premium = 4_000_000.0;
        let decision = syndicate.check_premium_exposure(moderate_premium);
        assert_eq!(
            decision,
            ExposureDecision::Accept,
            "Moderate premium should be accepted"
        );

        // Case 3: Premium at threshold
        // premium_to_capital = 5M / 10M = 0.5 = threshold → Accept
        syndicate.annual_premiums = 0.0;
        let threshold_premium = 5_000_000.0;
        let decision = syndicate.check_premium_exposure(threshold_premium);
        assert_eq!(
            decision,
            ExposureDecision::Accept,
            "Premium at threshold should be accepted"
        );

        // Case 4: Premium slightly over threshold
        // premium_to_capital = 6M / 10M = 0.6 > 0.5 → ScalePremium
        // excess_ratio = 0.6 / 0.5 = 1.2 < 2.0 → scale by 1.2
        syndicate.annual_premiums = 0.0;
        let over_threshold_premium = 6_000_000.0;
        let decision = syndicate.check_premium_exposure(over_threshold_premium);
        match decision {
            ExposureDecision::ScalePremium(factor) => {
                assert!(factor > 1.0, "Should scale premium up when over threshold");
                assert!(factor < 2.0, "Scale factor should be less than 2.0");
            }
            _ => panic!("Expected ScalePremium decision, got {:?}", decision),
        }

        // Case 5: Premium far over threshold
        // premium_to_capital = 11M / 10M = 1.1 > 0.5 → Reject
        // excess_ratio = 1.1 / 0.5 = 2.2 > 2.0 → Reject
        syndicate.annual_premiums = 0.0;
        let far_over_premium = 11_000_000.0;
        let decision = syndicate.check_premium_exposure(far_over_premium);
        assert_eq!(
            decision,
            ExposureDecision::Reject,
            "Premium far over threshold should be rejected"
        );

        // Case 6: Accumulated premium matters
        // Already have 3M annual premium, adding 3M more = 6M total
        // premium_to_capital = 6M / 10M = 0.6 > 0.5 → ScalePremium
        syndicate.annual_premiums = 3_000_000.0;
        let additional_premium = 3_000_000.0;
        let decision = syndicate.check_premium_exposure(additional_premium);
        match decision {
            ExposureDecision::ScalePremium(_) => {} // Expected
            ExposureDecision::Reject => {}          // Also acceptable if ratio is too high
            _ => panic!("Expected ScalePremium or Reject when accumulated premium is high"),
        }

        // Case 7: Negative or zero capital
        syndicate.capital = 0.0;
        let decision = syndicate.check_premium_exposure(100_000.0);
        assert_eq!(
            decision,
            ExposureDecision::Reject,
            "Should reject when capital is zero"
        );

        syndicate.capital = -1_000_000.0;
        let decision = syndicate.check_premium_exposure(100_000.0);
        assert_eq!(
            decision,
            ExposureDecision::Reject,
            "Should reject when capital is negative"
        );
    }
}
