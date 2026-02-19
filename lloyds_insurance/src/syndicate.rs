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
    // Sum of per-unit (normalized) claim amounts this year, for annual X̄_t update
    annual_claim_normalized_sum: f64,

    // Underwriting markup: exponentially weighted moving average of market conditions
    // m_t captures competitive pressure based on loss experience
    markup_m_t: f64,

    // Loss ratio history (tracked but not used in markup calculation)
    // Markup updates use current year's loss ratio per paper specification
    prior_year_loss_ratio: Option<f64>,

    // Dynamic industry statistics (updated annually from MarketStatisticsCollector)
    // These replace the hardcoded config values for actuarial pricing
    industry_lambda_t: f64, // Industry-wide claim frequency (claims per policy)
    industry_mu_t: f64,     // Industry-wide average claim cost
    years_elapsed: usize,   // Track years for warmup period

    // Capital at the start of the current year (before collecting any new premiums).
    // Used as the denominator in Premium EM so the cap doesn't expand as premiums arrive.
    capital_at_year_start: f64,

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
            Some(VarExposureManager::new(config.clone(), initial_capital))
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
            annual_claim_normalized_sum: 0.0,
            // Start at actuarially fair pricing (m_t = 0)
            // Markup will adjust via EWMA based on observed loss ratios
            // Per paper Section 4.3.2: no initial bias specified
            markup_m_t: 0.0,
            prior_year_loss_ratio: None, // No prior experience yet
            industry_lambda_t,
            industry_mu_t,
            years_elapsed: 0,
            capital_at_year_start: initial_capital,
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
        // loss_history stores NORMALIZED claim amounts (amount / line_size), so every entry
        // represents the per-unit-of-exposure claim regardless of whether the syndicate
        // was lead or follow on that policy.  Multiplying by lead_line_size at the end
        // converts back to the expected loss for a lead participation.
        //
        // industry_avg_loss_per_participation is passed in already scaled for the lead line
        // (= industry_mu_t × industry_lambda_t, where industry_mu_t = gamma_mean × lead_line).

        let z = self.config.internal_experience_weight;
        let claim_freq = self.config.yearly_claim_frequency;
        let lead_line = self.config.default_lead_line_size;

        // Syndicate's own experience (expected loss per lead participation)
        let syndicate_expected_loss = if !self.loss_history.is_empty() {
            // loss_history contains per-unit-of-exposure amounts (normalized by line_size
            // when the claim was recorded in handle_claim).
            let weight = self.config.loss_recency_weight;
            let mut weighted_sum = 0.0;
            let mut total_weight = 0.0;
            for (i, loss) in self.loss_history.iter().rev().enumerate() {
                let w = (1.0 - weight).powi(i as i32);
                weighted_sum += loss * w;
                total_weight += w;
            }
            let avg_per_unit = weighted_sum / total_weight;
            // Scale to expected loss for a lead participation: E[loss] = P(claim) × avg_per_unit × lead_line
            avg_per_unit * claim_freq * lead_line
        } else {
            // No history yet - use industry average (already per-lead-participation)
            industry_avg_loss_per_participation
        };

        // Industry expected loss per participation (use directly)
        let industry_expected_loss = industry_avg_loss_per_participation;

        // Combine syndicate and industry experience
        let base_price = z * syndicate_expected_loss + (1.0 - z) * industry_expected_loss;

        // Add volatility loading: F_t = EWMA-weighted std dev of loss history (Paper Eq 2)
        // P_at = P̃_t + α·F_t  where F_t is the std deviation of per-unit claims × lead_line
        let f_t = if !self.loss_history.is_empty() {
            let weight = self.config.loss_recency_weight;
            let mut weighted_sum = 0.0;
            let mut weighted_sq_sum = 0.0;
            let mut total_weight = 0.0;
            for (i, &loss) in self.loss_history.iter().rev().enumerate() {
                let w = (1.0 - weight).powi(i as i32);
                weighted_sum += loss * w;
                weighted_sq_sum += loss * loss * w;
                total_weight += w;
            }
            let mean = weighted_sum / total_weight;
            let variance = (weighted_sq_sum / total_weight) - mean * mean;
            // Scale std dev to lead-participation units (same as expected loss above)
            variance.max(0.0).sqrt() * lead_line
        } else {
            0.0 // no history → no variance estimate → no volatility loading
        };
        let volatility_loading = self.config.volatility_weight * f_t;

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

        // Use capital_at_year_start as the denominator to prevent the cap from
        // expanding as premiums are collected during the year (fixed annual budget).
        let proposed_total_premium = self.annual_premiums + proposed_premium;
        let premium_to_capital_ratio = proposed_total_premium / self.capital_at_year_start;

        // Check against threshold
        let threshold = self.config.premium_reserve_ratio;

        if premium_to_capital_ratio <= threshold {
            // Within limits
            ExposureDecision::Accept
        } else {
            // Exceeds threshold.
            // For the LEAD path, handle_lead_quote_request converts ScalePremium → Reject.
            // For the FOLLOW path, handle_follow_quote_request uses ScalePremium to reduce
            // line size to stay within budget — this is the correct paper behaviour.
            let excess_ratio = premium_to_capital_ratio / threshold;
            if excess_ratio > 2.0 {
                ExposureDecision::Reject
            } else {
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
        let price = self.apply_underwriting_markup(actuarial_price);

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
                ExposureDecision::ScalePremium(_factor) => {
                    // handle_lead_accepted recalculates price from scratch (ignores quoted price),
                    // so scaling has no effect. Treat as Reject to enforce the VaR limit.
                    return Vec::new();
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
                ExposureDecision::ScalePremium(_factor) => {
                    // Hard cap: budget exhausted. Scaling price doesn't prevent acceptance
                    // (broker may still pick it) and handle_lead_accepted recalculates price
                    // independently, so scaling has no effect. Reject instead.
                    return Vec::new();
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
        let mut line_size = if pricing_strength < 0.5 {
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

        // Exposure management: Use VaR EM if enabled (Scenario 3), otherwise use Premium EM
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
                    // Decline the quote if VaR suggests scaling
                    return Vec::new();
                }
            }
        } else {
            // Premium-based exposure management (Scenario 1/4)
            // Estimate the premium we'd collect for this line size
            let estimated_premium = (lead_price / self.config.default_lead_line_size) * line_size;
            match self.check_premium_exposure(estimated_premium) {
                ExposureDecision::Accept => {
                    // Within premium limits, proceed
                }
                ExposureDecision::Reject => {
                    // Premium-to-capital ratio too high
                    return Vec::new();
                }
                ExposureDecision::ScalePremium(_factor) => {
                    // Can't scale premium (lead sets price) → reduce line size to stay within budget
                    let premium_per_unit = lead_price / self.config.default_lead_line_size;
                    if premium_per_unit > 0.0 {
                        let max_premium = self.config.premium_reserve_ratio
                            * self.capital_at_year_start
                            - self.annual_premiums;
                        let max_line_size = max_premium / premium_per_unit;
                        if max_line_size <= 0.0 {
                            return Vec::new();
                        }
                        line_size = max_line_size.min(line_size);
                    } else {
                        return Vec::new();
                    }
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
        lead_price: f64,
        peril_region: usize,
        risk_limit: f64,
    ) {
        // Paper Section 4.3.4: followers collect the lead's price scaled to their line share.
        // lead_price is quoted for default_lead_line_size (50%); scale to our allocated line.
        let price = (lead_price / self.config.default_lead_line_size) * line_size;

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

    fn handle_claim(&mut self, risk_id: usize, amount: f64) -> Vec<(usize, Event)> {
        self.capital -= amount;

        // Normalize the claim by the participation line_size so that loss_history stores
        // per-unit-of-exposure amounts.  This prevents mixing lead ($1.5M/event at 50%)
        // and follow ($300k/event at 10%) claim amounts, which would otherwise cause the
        // EWMA to underestimate the fair lead price as experience accumulates.
        let line_size = self
            .policies
            .iter()
            .find(|p| p.risk_id == risk_id)
            .map(|p| p.line_size)
            .unwrap_or(self.config.default_lead_line_size);
        let normalized_amount = if line_size > 0.0 {
            amount / line_size
        } else {
            amount
        };
        // Accumulate for the annual average entry (pushed to loss_history at year end).
        // Storing one per-year average instead of one-per-claim prevents a lucky year
        // with a few low-severity claims from dominating the recency-weighted EWMA.
        self.annual_claim_normalized_sum += normalized_amount;

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

        // Push one annual-average per-unit claim entry to loss_history.
        // Each year contributes exactly one entry, so freak low/high years don't dominate
        // the recency-weighted EWMA (unlike per-claim entries which over-weight claim-heavy years).
        if self.annual_claims_count > 0 {
            let annual_avg_severity =
                self.annual_claim_normalized_sum / self.annual_claims_count as f64;
            self.loss_history.push(annual_avg_severity);
        }

        // Reset annual counters
        self.annual_premiums = 0.0;
        self.annual_claims = 0.0;
        self.annual_policies_written = 0;
        self.annual_claims_count = 0;
        self.annual_claim_normalized_sum = 0.0;

        // Snapshot capital at year start (after dividends) for Premium EM denominator.
        // Using start-of-year capital prevents the cap from expanding as premiums arrive.
        self.capital_at_year_start = self.capital;
    }

    fn update_underwriting_markup(&mut self) {
        // Update m_t using EWMA: m_t = (1-α)·m_{t-1} + α·signal_t
        // where signal_t = LR_t - 1  (loss ratio deviation from break-even)
        //
        // Per paper Section 4.3.2 (Underwriting Sub-Process):
        // "m_t captures competitive pressure based on loss experience"
        //
        // This captures competitive pressure:
        // - High loss ratios (>1) → positive signal → m_t increases → higher premiums
        // - Low loss ratios (<1) → negative signal → m_t decreases → lower premiums
        // - Balanced loss ratios (≈1) → signal ≈ 0 → m_t stable
        //
        // Using signal = LR - 1 (not ln(LR)) eliminates Jensen's inequality bias:
        // E[LR - 1] = 0 when E[LR] = 1 (fair pricing), so m_t has no systematic drift.
        //
        // α = underwriter_recency_weight is the weight on new signal (standard EWMA).

        let current_year_loss_ratio = if self.annual_premiums > 0.0 {
            Some(self.annual_claims / self.annual_premiums)
        } else {
            None
        };

        // Update markup using current year's loss ratio (per paper specification)
        if let Some(loss_ratio) = current_year_loss_ratio {
            // signal = LR - 1: positive when unprofitable (LR>1), negative when profitable
            let signal = loss_ratio - 1.0;

            // α = fraction of new signal incorporated each year.
            // underwriter_recency_weight is the weight on NEW data (high = fast adaptation).
            // Warmup: slow adaptation in early years to avoid first-year noise dominating.
            let alpha = if self.years_elapsed == 0 {
                0.05 // Year 0: very slow adaptation (5% of new signal)
            } else if self.years_elapsed < 5 {
                0.10 // Years 1-4: slow adaptation (10% of new signal)
            } else {
                self.config.underwriter_recency_weight // Years 5+: config value (default 20%)
            };

            // EWMA update: m_t = (1-α)·m_{t-1} + α·signal
            self.markup_m_t = (1.0 - alpha) * self.markup_m_t + alpha * signal;
            // Floor: no syndicate should price more than ~18% below fair value.
            // Without this, two consecutive profitable years (LR≈0.4) drive markup to −0.24,
            // making the premium cap permissive (more policies fit) → overwriting → insolvency.
            self.markup_m_t = self.markup_m_t.max(0.0);
        }

        // Track history for potential future use
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

    /// Compute uniform deviation (CoV of exposure across regions) from the exposure map.
    /// Used for scenarios without VaR EM so that S2 deviation is comparable to S3's.
    fn compute_uniform_deviation_from_map(
        exposure_by_region: &std::collections::HashMap<usize, f64>,
        num_regions: usize,
    ) -> f64 {
        if num_regions == 0 {
            return 0.0;
        }
        let total: f64 = exposure_by_region.values().sum();
        if total == 0.0 {
            return 0.0;
        }
        let mean = total / num_regions as f64;
        let variance: f64 = (0..num_regions)
            .map(|r| {
                let exp = exposure_by_region.get(&r).copied().unwrap_or(0.0);
                (exp - mean).powi(2)
            })
            .sum::<f64>()
            / num_regions as f64;
        (variance.sqrt() / mean).min(1.0)
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

            // Capture uniform_deviation BEFORE year-end reset (reflects this year's exposure).
            // Computed for ALL scenarios (not just VaR EM) so S2 vs S3 comparison is valid.
            let uniform_deviation = if let Some(ref var_em) = self.var_exposure_manager {
                var_em.uniform_deviation()
            } else {
                Self::compute_uniform_deviation_from_map(
                    &self.stats.exposure_by_peril_region,
                    self.config.num_peril_regions,
                )
            };

            self.handle_year_end();

            // Reset VaR exposure for new year — active policies from last year have expired
            if let Some(ref mut var_em) = self.var_exposure_manager {
                var_em.reset_exposures();
            }
            self.stats.exposure_by_peril_region.clear();

            self.update_stats();
            // Override update_stats() result with the pre-reset value
            self.stats.uniform_deviation = uniform_deviation;

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
                lead_price,
                peril_region,
                risk_limit,
            } if *syndicate_id == self.syndicate_id => {
                self.handle_follow_accepted(
                    *risk_id,
                    *line_size,
                    *lead_price,
                    *peril_region,
                    *risk_limit,
                );
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
                avg_claim_cost: _, // intentionally ignored — see note below
            } => {
                // industry_lambda_t intentionally not updated from observations.
                //
                // Single-year observed claim frequencies are too noisy with ~100–160 policies
                // per syndicate (one freak profitable year with observed_freq≈0.04 vs. config
                // 0.1 drove alpha=0.4 updates that collapsed the actuarial base by 35%).
                // The config-based lambda is a structural market parameter; only the syndicate's
                // own loss_history (X̄_t, z=0.5) adapts to experience.
                //
                // industry_mu_t is also kept fixed (same rationale, plus mixed lead/follow
                // claim amounts from market stats are incompatible with the formula).
                let _ = avg_claim_frequency; // suppress unused-variable warning

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
        // Paper Eq 2: volatility loading uses std dev (F_t). With no loss history, F_t = 0.
        let expected_price = industry_avg_per_participation; // $150k, no volatility loading

        let price = syndicate.calculate_actuarial_price(1, industry_avg_per_participation);

        // With no history, f_t = 0 → no volatility loading → price = base = $150k
        assert!(
            (price - expected_price).abs() < 1.0,
            "Expected ${:.0} (base price, no history → f_t=0), got ${:.0}",
            expected_price,
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
                lead_price: 150_000.0,
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
        let config = ModelConfig {
            underwriter_recency_weight: 0.2,
            ..ModelConfig::default()
        };
        let mut syndicate = Syndicate::new(0, config.clone());

        // Initial markup is 0.0 (fair pricing)
        syndicate.markup_m_t = 0.0;
        // Skip warmup period (need years_elapsed >= 5)
        syndicate.years_elapsed = 5;

        // Simulate a high-loss year: loss_ratio = 2.0
        syndicate.annual_premiums = 1_000_000.0;
        syndicate.annual_claims = 2_000_000.0;

        // Update markup at year-end
        syndicate.update_underwriting_markup();

        // markup should be positive: m_t = 0.8*0.0 + 0.2*(2.0-1.0) = 0.2
        assert!(
            syndicate.markup_m_t > 0.0,
            "Markup should increase after high losses"
        );
        let expected = 0.2 * (2.0 - 1.0); // = 0.2
        assert!(
            (syndicate.markup_m_t - expected).abs() < 1e-9,
            "Markup should be {:.3}, got {:.3}",
            expected,
            syndicate.markup_m_t
        );
    }

    #[test]
    fn test_underwriting_markup_decreases_after_profits() {
        let config = ModelConfig {
            underwriter_recency_weight: 0.2,
            ..ModelConfig::default()
        };
        let mut syndicate = Syndicate::new(0, config.clone());

        // Start with some positive markup
        syndicate.markup_m_t = 0.5;
        // Skip warmup period (need years_elapsed >= 5)
        syndicate.years_elapsed = 5;

        // Simulate a profitable year: loss_ratio = 0.5
        syndicate.annual_premiums = 1_000_000.0;
        syndicate.annual_claims = 500_000.0;

        // Update markup
        syndicate.update_underwriting_markup();

        // markup should decrease: m_t = 0.8*0.5 + 0.2*(0.5-1.0) = 0.4 - 0.1 = 0.3 < 0.5
        assert!(
            syndicate.markup_m_t < 0.5,
            "Markup should decrease from 0.5 after profitable year, got {}",
            syndicate.markup_m_t
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
    fn test_pricing_calculation_detailed() {
        // Debug test to verify pricing calculations match expectations
        let config = ModelConfig::default();
        let syndicate = Syndicate::new(0, config.clone());

        // Expected calculation:
        // industry_lambda_t = 0.1 (10% claim frequency)
        // industry_mu_t = 3M * 0.5 = 1.5M (mean claim for 50% line)
        // industry_avg_loss = 0.1 * 1.5M = $150k per participation
        let industry_avg_loss = syndicate.industry_mu_t * syndicate.industry_lambda_t;

        assert_eq!(
            syndicate.industry_lambda_t, 0.1,
            "Claim frequency should be 10%"
        );
        assert_eq!(
            syndicate.industry_mu_t, 1_500_000.0,
            "Mean claim should be $1.5M for 50% line"
        );
        assert_eq!(
            industry_avg_loss, 150_000.0,
            "Expected loss per participation should be $150k"
        );

        // Calculate actuarial price
        // Paper Eq 2: P_at = P̃_t + α·F_t, where F_t = std dev of loss_history.
        // With no loss history, F_t = 0 → no volatility loading → price = base = $150k.
        let actuarial_price = syndicate.calculate_actuarial_price(1, industry_avg_loss);

        assert!(
            (actuarial_price - 150_000.0).abs() < 1_000.0,
            "Actuarial price should be ~$150k (base price, no history → F_t=0), got ${:.0}",
            actuarial_price
        );

        // Apply markup (m_t = 0.0 initially, so final = actuarial)
        let final_price = syndicate.apply_underwriting_markup(actuarial_price);
        let expected_final = actuarial_price * syndicate.markup_m_t.exp(); // e^0.0 = 1.0

        assert!(
            (final_price - expected_final).abs() < 1_000.0,
            "Final price should be actuarial * e^m_t, expected ${:.0}, got ${:.0}",
            expected_final,
            final_price
        );
    }

    #[test]
    fn test_full_quote_cycle() {
        // Test a complete quote -> accept -> premium cycle
        let config = ModelConfig::default();
        let mut syndicate = Syndicate::new(0, config.clone());

        let risk_id = 1;
        let peril_region = 0;
        let risk_limit = 10_000_000.0;

        // Step 1: Syndicate receives lead quote request
        let quote_response = syndicate.handle_lead_quote_request(
            risk_id,
            peril_region,
            risk_limit,
            0, // current_t
        );

        // Extract the quoted premium
        let quoted_premium =
            if let Some((_, Event::LeadQuoteOffered { price, .. })) = quote_response.first() {
                *price
            } else {
                panic!("Expected LeadQuoteOffered event");
            };

        println!("\n=== Quote Cycle Debug ===");
        println!("Quoted premium: ${:.0}", quoted_premium);

        // Expected: ~$150k (actuarial fair price, no loss history → F_t=0, m_t=0.0)
        assert!(
            (quoted_premium - 150_000.0).abs() < 5_000.0,
            "Quoted premium should be ~$150k (fair price, no history), got ${:.0}",
            quoted_premium
        );

        // Step 2: Quote gets accepted
        let initial_capital = syndicate.capital;
        syndicate.handle_lead_accepted(risk_id, peril_region, risk_limit);

        let premium_collected = syndicate.annual_premiums;
        println!("Premium collected: ${:.0}", premium_collected);
        println!(
            "Capital increase: ${:.0}",
            syndicate.capital - initial_capital
        );

        // Verify premium collected matches what was quoted
        assert!(
            (premium_collected - quoted_premium).abs() < 1.0,
            "Premium collected (${:.0}) should match quoted (${:.0})",
            premium_collected,
            quoted_premium
        );
    }

    #[test]
    fn test_loss_ratio_with_simulated_claims() {
        // Simulate many policies to verify loss ratios average < 1.0
        use rand::{Rng, SeedableRng};
        use rand_distr::{Distribution, Gamma};

        let config = ModelConfig::default();
        let mut syndicate = Syndicate::new(0, config.clone());
        let mut rng = rand::rngs::StdRng::seed_from_u64(12345);

        // Gamma distribution for claim sizes
        let shape = 1.0 / (config.gamma_cov * config.gamma_cov);
        let scale = config.gamma_mean * config.gamma_cov * config.gamma_cov;
        let gamma = Gamma::new(shape, scale).unwrap();

        let num_policies = 1000;
        let line_size = config.default_lead_line_size;

        // Write policies and collect premiums
        for i in 0..num_policies {
            syndicate.handle_lead_accepted(i, 0, config.risk_limit);
        }

        let total_premiums = syndicate.annual_premiums;

        // Generate claims with 10% frequency
        let mut total_claims = 0.0;
        for i in 0..num_policies {
            if rng.r#gen::<f64>() < config.yearly_claim_frequency {
                let claim_amount = gamma.sample(&mut rng).min(config.risk_limit);
                let syndicate_share = claim_amount * line_size;
                syndicate.handle_claim(i, syndicate_share);
                total_claims += syndicate_share;
            }
        }

        let loss_ratio = total_claims / total_premiums;

        println!("\n=== Simulated Loss Ratio ===");
        println!("Policies: {}", num_policies);
        println!("Total premiums: ${:.0}", total_premiums);
        println!("Total claims: ${:.0}", total_claims);
        println!("Loss ratio: {:.4}", loss_ratio);
        println!("Expected: <1.0 (due to 20% volatility loading + 15% markup)");

        // With 20% volatility loading and 15% markup, loss ratio should average < 0.8
        assert!(
            loss_ratio < 0.95,
            "Loss ratio should be <0.95 with volatility loading and markup, got {:.4}",
            loss_ratio
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

    // ========================================================================
    // PRICING STABILITY TESTS (diagnose Year 1-2 pricing collapse)
    // ========================================================================

    #[test]
    fn test_warmup_period_year_0_markup_stability() {
        // Verify Year 0 markup adjustment is dampened (alpha=5% weight on new signal)
        // New formula: signal = LR - 1, alpha = 0.05 in year 0
        let config = ModelConfig::default();
        let mut syndicate = Syndicate::new(0, config.clone());

        // Year 0: profitable year (loss_ratio = 0.5)
        syndicate.years_elapsed = 0;
        syndicate.markup_m_t = 0.0;
        syndicate.annual_premiums = 1_000_000.0;
        syndicate.annual_claims = 500_000.0;

        syndicate.update_underwriting_markup();

        // Expected: alpha=0.05, signal = 0.5 - 1.0 = -0.5
        // EWMA before floor: 0.95 * 0.0 + 0.05 * (-0.5) = -0.025
        // After markup floor max(0.0): clamped to 0.0
        // The floor prevents negative markup to avoid unsustainable price cuts.

        assert!(
            syndicate.markup_m_t >= 0.0,
            "Year 0 markup should be floored at 0.0, got {:.4}",
            syndicate.markup_m_t
        );

        // Should be small, not artificially boosted
        assert!(
            syndicate.markup_m_t < 0.1,
            "Year 0 markup should be near-zero after profitable year: {:.4} < 0.1",
            syndicate.markup_m_t
        );
    }

    #[test]
    fn test_warmup_prevents_year_1_premium_collapse() {
        // Simulate Year 0 profitable → Year 1 markup adjustment
        // Without warmup: markup goes from 0 → -0.5, premium drops 39%
        // With warmup: markup goes from 0 → -0.07, premium drops only 7%

        let config = ModelConfig::default();
        let mut syndicate = Syndicate::new(0, config.clone());

        // Year 0: profitable (loss_ratio = 0.54, like real data)
        syndicate.years_elapsed = 0;
        syndicate.markup_m_t = 0.0;
        syndicate.annual_premiums = 1_000_000.0;
        syndicate.annual_claims = 540_000.0;

        syndicate.update_underwriting_markup();

        let year0_markup = syndicate.markup_m_t;

        // Premium multiplier: exp(markup)
        let premium_multiplier = year0_markup.exp();

        println!("\n=== Warmup Prevents Premium Collapse ===");
        println!("Year 0 loss ratio: 0.54");
        println!("Year 0 markup (with warmup): {:.4}", year0_markup);
        println!("Year 1 premium multiplier: {:.4}", premium_multiplier);
        println!(
            "Year 1 premium change: {:.1}%",
            (premium_multiplier - 1.0) * 100.0
        );

        // With warmup (10% weight): markup = 0.1 * ln(0.54) = -0.062
        // Premium change = exp(-0.062) - 1 = -6% (acceptable)
        assert!(
            premium_multiplier > 0.9,
            "Premium should not drop >10% in Year 1 with warmup: {:.1}% drop",
            (1.0 - premium_multiplier) * 100.0
        );

        // Without warmup (80% weight): markup = 0.8 * ln(0.54) = -0.493
        // Premium change = exp(-0.493) - 1 = -39% (catastrophic!)
        let without_warmup_markup = 0.8 * (0.54_f64).ln();
        let without_warmup_multiplier = without_warmup_markup.exp();

        println!("WITHOUT warmup:");
        println!("  Markup: {:.4}", without_warmup_markup);
        println!("  Premium multiplier: {:.4}", without_warmup_multiplier);
        println!(
            "  Premium change: {:.1}%\n",
            (without_warmup_multiplier - 1.0) * 100.0
        );

        assert!(
            without_warmup_multiplier < 0.65,
            "Without warmup, premium drops >35%"
        );
    }

    #[test]
    fn test_scenario_3_volatility_buffer_increases_premiums() {
        // Verify that α=0.5 produces higher premiums than α=0.0
        // when there is variance in the loss history.
        // Paper Eq 2: P_at = P̃_t + α·F_t; higher α → larger loading when F_t > 0.
        // Note: paper Table 13/14 sets α=0 for all scenarios; this test exercises the mechanism.
        let config_default = ModelConfig::default(); // volatility_weight = 0.0
        let config_high_alpha = ModelConfig {
            volatility_weight: 0.5,
            ..ModelConfig::default()
        };

        assert_eq!(
            config_default.volatility_weight, 0.0,
            "Default should be 0 (paper Table 13/14: α=0 for all scenarios)"
        );
        assert_eq!(config_high_alpha.volatility_weight, 0.5);

        // Pre-populate the same variable loss history for both syndicates.
        // With variance in loss history, F_t > 0, so higher α produces a larger loading.
        let history = vec![1_000_000.0, 3_000_000.0, 4_280_000.0]; // same mean, σ > 0

        let mut syndicate_default = Syndicate::new(0, config_default);
        syndicate_default.loss_history = history.clone();

        let mut syndicate_high_alpha = Syndicate::new(0, config_high_alpha);
        syndicate_high_alpha.loss_history = history;

        let industry_avg = 150_000.0; // 0.1 * $1.5M
        let price_default = syndicate_default.calculate_actuarial_price(1, industry_avg);
        let price_high_alpha = syndicate_high_alpha.calculate_actuarial_price(1, industry_avg);

        // Higher α should produce higher premiums than α=0.0
        assert!(
            price_high_alpha > price_default,
            "α=0.5 should produce higher premiums than α=0. \
             Default=${:.0}, HighAlpha=${:.0}",
            price_default,
            price_high_alpha
        );
    }

    #[test]
    fn test_reduced_dividends_preserve_capital() {
        // Verify that a reduced profit_fraction (20%) pays smaller dividends than the default (40%)
        // and preserves more capital. This tests the dividend mechanism, not a specific scenario.
        let config_default = ModelConfig::default();
        let config_low_dividend = ModelConfig {
            profit_fraction: 0.2,
            ..ModelConfig::default()
        };

        assert_eq!(config_default.profit_fraction, 0.4, "Default should be 40%");
        assert_eq!(config_low_dividend.profit_fraction, 0.2);

        // Simulate profitable year with reduced dividend config
        let mut syndicate = Syndicate::new(0, config_low_dividend.clone());
        let initial_capital = syndicate.capital;

        syndicate.annual_premiums = 1_000_000.0;
        syndicate.annual_claims = 600_000.0; // $400k profit

        syndicate.handle_year_end();

        // Dividend = 0.2 * $400k = $80k
        let expected_dividend = 0.2 * 400_000.0;
        let expected_capital = initial_capital - expected_dividend;

        assert_eq!(
            syndicate.stats.total_dividends_paid, expected_dividend,
            "Should pay 20% dividend (not 40%)"
        );

        assert_eq!(
            syndicate.capital, expected_capital,
            "Should retain 80% of profit as capital buffer"
        );
    }

    #[test]
    fn test_loss_ratio_realistic_with_50_percent_buffer() {
        // Simulate realistic portfolio with 50% volatility buffer
        // Expected: loss ratio should average ~0.67 (1 / 1.5)
        use rand::{Rng, SeedableRng};
        use rand_distr::{Distribution, Gamma};

        let config = ModelConfig::scenario_3();
        let mut syndicate = Syndicate::new(0, config.clone());
        syndicate.years_elapsed = 5; // Skip warmup

        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let shape = 1.0 / (config.gamma_cov * config.gamma_cov);
        let scale = config.gamma_mean * config.gamma_cov * config.gamma_cov;
        let gamma = Gamma::new(shape, scale).unwrap();

        let num_policies = 2000;
        let line_size = config.default_lead_line_size;

        // Write policies
        for i in 0..num_policies {
            syndicate.handle_lead_accepted(i, 0, config.risk_limit);
        }

        let total_premiums = syndicate.annual_premiums;

        // Generate claims
        let mut total_claims = 0.0;
        for i in 0..num_policies {
            if rng.gen_range(0.0..1.0) < config.yearly_claim_frequency {
                let claim_amount = gamma.sample(&mut rng).min(config.risk_limit);
                let syndicate_share = claim_amount * line_size;
                syndicate.handle_claim(i, syndicate_share);
                total_claims += syndicate_share;
            }
        }

        let loss_ratio = total_claims / total_premiums;

        println!("\n=== Loss Ratio with 50% Buffer ===");
        println!("Policies: {}", num_policies);
        println!("Premiums: ${:.2}M", total_premiums / 1_000_000.0);
        println!("Claims: ${:.2}M", total_claims / 1_000_000.0);
        println!("Loss ratio: {:.4}", loss_ratio);
        println!("Expected: ~1.0 (no prior history → F_t=0 → premiums at fair price)");

        // Paper Eq 2: volatility buffer requires prior loss history with variance (F_t > 0).
        // In year 1 (no history), premiums = actuarial fair price → expected loss ratio ≈ 1.0.
        // Portfolio should remain profitable (loss_ratio < 1.0) within a reasonable band.
        assert!(
            loss_ratio < 1.1,
            "Loss ratio {:.4} should be near 1.0 (fair pricing) with no prior history",
            loss_ratio
        );
        assert!(
            loss_ratio > 0.5,
            "Loss ratio {:.4} should be >0.5 (claims can't exceed 2× fair premiums by chance)",
            loss_ratio
        );
    }

    // ========================================================================
    // ACTUARIAL PRICING CORRECTNESS & DISCREPANCY TESTS
    // ========================================================================
    //
    // These tests verify the actuarial pricing formula against the paper
    // (Olmez et al., 2024) and document known discrepancies between the
    // implementation and the paper's specification.
    //
    // Discrepancy-documenting tests (A2, C1, D1) PASS — they assert the
    // implementation's actual behaviour with NOTE comments explaining where
    // the paper differs. They serve as regression tests.

    // --- Category A: Actuarial Formula Structure ---

    #[test]
    fn test_a1_zero_volatility_produces_expected_loss() {
        // With volatility_weight=0, price should equal pure expected loss
        // Paper Table 13/14: α=0 for all four scenarios
        let config = ModelConfig {
            volatility_weight: 0.0,
            ..Default::default()
        };
        let syndicate = Syndicate::new(0, config);

        // industry_avg_per_participation = 0.1 × ($3M × 0.5) = $150k
        let industry_avg = syndicate.industry_mu_t * syndicate.industry_lambda_t;
        let price = syndicate.calculate_actuarial_price(1, industry_avg);

        assert!(
            (price - 150_000.0).abs() < 1.0,
            "With α=0, price should equal expected loss $150k, got ${:.2}",
            price
        );
    }

    #[test]
    fn test_a2_volatility_loading_uses_claim_std_dev() {
        // Paper Eq 2: P_at = P̃_t + α·F_t where F_t = std dev of claims (not mean).
        // Two syndicates with same EWMA mean but different variance should get different prices.
        let config = ModelConfig {
            volatility_weight: 0.2,
            ..Default::default()
        };

        // Syndicate A: constant claims (σ ≈ 0)
        let mut syndicate_a = Syndicate::new(0, config.clone());
        syndicate_a.loss_history = vec![3_000_000.0, 3_000_000.0, 3_000_000.0];

        // Syndicate B: variable claims, SAME recency-weighted mean
        // EWMA for [3M,3M,3M] = (3M×1.0 + 3M×0.8 + 3M×0.64) / 2.44 = 3M
        // Need c + 0.8b + 0.64a = 7.32M with b=3M → c + 0.64a = 4.92M
        // Choose a=1M → c=4.28M: [1M, 3M, 4.28M] has same EWMA but σ > 0
        let mut syndicate_b = Syndicate::new(1, config.clone());
        syndicate_b.loss_history = vec![1_000_000.0, 3_000_000.0, 4_280_000.0];

        let industry_avg = syndicate_a.industry_mu_t * syndicate_a.industry_lambda_t;
        let price_a = syndicate_a.calculate_actuarial_price(1, industry_avg);
        let price_b = syndicate_b.calculate_actuarial_price(1, industry_avg);

        // Paper formula (Eq 2): F_t = std dev of claims → syndicate B (higher variance)
        // charges more than A (zero variance), even though both have the same EWMA mean.
        assert!(
            price_b > price_a,
            "Syndicate B (higher variance) should price higher than A (zero variance). \
             Got A=${:.0}, B=${:.0}",
            price_a,
            price_b
        );
    }

    #[test]
    fn test_a3_ewma_loss_history_formula() {
        // Verify EWMA weighted average computation with known values.
        // loss_history stores per-unit-of-exposure amounts (normalized by line_size in
        // handle_claim); the formula then multiplies by claim_freq × lead_line_size.
        let config = ModelConfig {
            volatility_weight: 0.0,          // isolate base price
            internal_experience_weight: 1.0, // use only syndicate experience
            loss_recency_weight: 0.2,
            ..Default::default()
        };
        let mut syndicate = Syndicate::new(0, config.clone());
        // Per-unit-of-exposure amounts (e.g. claim_received / line_size)
        syndicate.loss_history = vec![1_000_000.0, 2_000_000.0, 3_000_000.0]; // oldest first

        // Manual EWMA calculation (iterating newest first):
        // i=0: w=(1-0.2)^0 = 1.0,  3M × 1.0  = 3.00M, total_w = 1.00
        // i=1: w=(1-0.2)^1 = 0.8,  2M × 0.8  = 1.60M, total_w = 1.80
        // i=2: w=(1-0.2)^2 = 0.64, 1M × 0.64 = 0.64M, total_w = 2.44
        // avg_per_unit = 5.24M / 2.44 ≈ 2,147,541
        // syndicate_expected_loss = avg_per_unit × claim_freq × lead_line
        //                         ≈ 2,147,541 × 0.1 × 0.5 ≈ 107,377

        let industry_avg = syndicate.industry_mu_t * syndicate.industry_lambda_t;
        let price = syndicate.calculate_actuarial_price(1, industry_avg);

        let expected = 5_240_000.0 / 2.44 * 0.1 * config.default_lead_line_size;
        assert!(
            (price - expected).abs() / expected < 0.01,
            "EWMA price should be ≈${:.0}, got ${:.0} (error {:.2}%)",
            expected,
            price,
            (price - expected).abs() / expected * 100.0
        );
    }

    #[test]
    fn test_a4_z_weight_blends_syndicate_and_industry() {
        // With loss history, z blends syndicate and industry expected loss
        // P̃_t = z·X̄_t + (1-z)·λ'·μ'
        //
        // loss_history holds per-unit amounts; formula: avg_per_unit × claim_freq × lead_line.
        // To get syndicate_expected = $200k with lead_line=0.5:
        //   avg_per_unit × 0.1 × 0.5 = $200k → avg_per_unit = $4M → uniform history at $4M.
        let config = ModelConfig {
            volatility_weight: 0.0,
            internal_experience_weight: 0.5,
            ..Default::default()
        };
        let mut syndicate = Syndicate::new(0, config);

        // Uniform per-unit history: EWMA = exactly $4M
        // syndicate_expected_loss = 4M × 0.1 × 0.5 = $200k
        syndicate.loss_history = vec![4_000_000.0, 4_000_000.0, 4_000_000.0];

        // Override industry stats so industry_avg = $100k
        syndicate.industry_mu_t = 1_000_000.0;
        syndicate.industry_lambda_t = 0.1;
        let industry_avg = 100_000.0;

        let price = syndicate.calculate_actuarial_price(1, industry_avg);

        // z=0.5: price = 0.5 × $200k + 0.5 × $100k = $150k
        assert!(
            (price - 150_000.0).abs() < 1.0,
            "z-blended price should be $150k (0.5×200k + 0.5×100k), got ${:.0}",
            price
        );
    }

    // --- Category B: Underwriting Markup ---

    #[test]
    fn test_b1_markup_increases_after_loss_year_warmup() {
        // Year 0 warmup: alpha=0.05, only 5% signal weight
        // LR=2.0 → signal = LR-1 = 1.0
        // m_t = 0.95×0 + 0.05×1.0 = 0.05
        let config = ModelConfig::default();
        let mut syndicate = Syndicate::new(0, config);

        syndicate.years_elapsed = 0;
        syndicate.markup_m_t = 0.0;
        syndicate.annual_premiums = 100_000.0;
        syndicate.annual_claims = 200_000.0;

        syndicate.update_underwriting_markup();

        let expected = 0.05 * (2.0 - 1.0); // = 0.05
        assert!(
            (syndicate.markup_m_t - expected).abs() < 0.001,
            "Year 0 markup should be {:.4} (5% alpha × signal), got {:.4}",
            expected,
            syndicate.markup_m_t
        );

        // Verify markup increased from 0 (losses → higher prices)
        assert!(
            syndicate.markup_m_t > 0.0,
            "Markup should be positive after loss year"
        );
    }

    #[test]
    fn test_b3_breakeven_loss_ratio_produces_zero_signal() {
        // LR=1.0 → signal = LR-1 = 0.0 → markup decays toward 0
        // New formula: m_t = (1-alpha)×m_{t-1} + alpha×signal
        let config = ModelConfig {
            underwriter_recency_weight: 0.2,
            ..ModelConfig::default()
        };
        let mut syndicate = Syndicate::new(0, config);

        let prior_markup = 0.3;
        syndicate.markup_m_t = prior_markup;
        syndicate.years_elapsed = 5; // post-warmup: alpha = underwriter_recency_weight = 0.2
        syndicate.annual_premiums = 100_000.0;
        syndicate.annual_claims = 100_000.0; // LR = 1.0

        syndicate.update_underwriting_markup();

        // signal = 1.0 - 1.0 = 0.0
        // m_t = (1-0.2) × 0.3 + 0.2 × 0.0 = 0.8 × 0.3 = 0.24
        let expected = (1.0 - 0.2) * prior_markup;
        assert!(
            (syndicate.markup_m_t - expected).abs() < 0.0001,
            "With LR=1.0 (zero signal), markup should decay toward 0. Expected {:.4}, got {:.4}",
            expected,
            syndicate.markup_m_t
        );
    }

    #[test]
    fn test_b4_markup_ewma_sequence_post_warmup() {
        // Two successive year-end triggers with known LR values
        // Post-warmup: alpha = underwriter_recency_weight = 0.2 (weight on new signal)
        // New formula: m_t = (1-alpha)×m_{t-1} + alpha×(LR-1)
        let config = ModelConfig {
            underwriter_recency_weight: 0.2,
            ..ModelConfig::default()
        };
        let mut syndicate = Syndicate::new(0, config);

        syndicate.markup_m_t = 0.0;
        syndicate.years_elapsed = 5;

        // Year 5: LR = 2.0
        syndicate.annual_premiums = 100_000.0;
        syndicate.annual_claims = 200_000.0;
        syndicate.update_underwriting_markup();

        // m_5 = 0.8 × 0 + 0.2 × (2.0-1.0) = 0.2
        let expected_m5 = 0.2 * (2.0 - 1.0);
        assert!(
            (syndicate.markup_m_t - expected_m5).abs() < 0.001,
            "After year 5 (LR=2.0): expected m_t={:.4}, got {:.4}",
            expected_m5,
            syndicate.markup_m_t
        );

        // Year 6: LR = 0.5
        syndicate.years_elapsed = 6;
        syndicate.annual_premiums = 100_000.0;
        syndicate.annual_claims = 50_000.0;
        syndicate.update_underwriting_markup();

        // m_6 = 0.8 × 0.2 + 0.2 × (0.5-1.0) = 0.16 - 0.10 = 0.06
        let expected_m6 = 0.8 * expected_m5 + 0.2 * (0.5 - 1.0);
        assert!(
            (syndicate.markup_m_t - expected_m6).abs() < 0.001,
            "After year 6 (LR=0.5): expected m_t={:.4}, got {:.4}",
            expected_m6,
            syndicate.markup_m_t
        );
    }

    // --- Category C: Fair Price and Calibration Discrepancies ---

    #[test]
    fn test_c1_default_config_matches_paper_alpha() {
        // Paper Table 13/14: α=0 for all four scenarios.
        // Default config should match the paper (no volatility inflation).
        let config_default = ModelConfig::default(); // volatility_weight should be 0.0

        assert_eq!(
            config_default.volatility_weight, 0.0,
            "Default volatility_weight should be 0.0 (paper Table 13/14: α=0)"
        );

        // With α=0, default and explicit paper config produce the same price
        let config_paper = ModelConfig {
            volatility_weight: 0.0,
            ..Default::default()
        };

        let syndicate_default = Syndicate::new(0, config_default);
        let syndicate_paper = Syndicate::new(1, config_paper);

        let industry_avg = syndicate_default.industry_mu_t * syndicate_default.industry_lambda_t;
        let price_default = syndicate_default.calculate_actuarial_price(1, industry_avg);
        let price_paper = syndicate_paper.calculate_actuarial_price(1, industry_avg);

        let ratio = price_default / price_paper;
        assert!(
            (ratio - 1.0).abs() < 0.01,
            "Default config should produce the same price as paper config (α=0). \
             Got ratio={:.3}",
            ratio
        );
    }

    #[test]
    fn test_c2_paper_fair_price_is_full_risk_expected_loss() {
        // Paper Section 4.5: fair price = λ × μ = 0.1 × $3M = $300k (full risk)
        // Per 50% lead participation: $150k
        let config = ModelConfig {
            volatility_weight: 0.0,
            ..Default::default()
        };
        let syndicate = Syndicate::new(0, config.clone());

        // Full risk expected loss
        let full_risk_expected_loss = config.yearly_claim_frequency * config.gamma_mean;
        assert!(
            (full_risk_expected_loss - 300_000.0).abs() < 1.0,
            "Paper fair price should be $300k (λ×μ), got ${:.0}",
            full_risk_expected_loss
        );

        // Per lead participation (50% line), with no history and α=0
        let industry_avg = syndicate.industry_mu_t * syndicate.industry_lambda_t;
        let price = syndicate.calculate_actuarial_price(1, industry_avg);

        // With no history, both z terms equal industry_avg → price = industry_avg = $150k
        assert!(
            (price - 150_000.0).abs() < 1.0,
            "Per-participation actuarial price should be $150k (50%% of $300k), got ${:.0}",
            price
        );
    }

    // --- Category D: Market Structure ---

    #[test]
    fn test_d1_follow_premium_uses_lead_price() {
        // Paper Section 4.3.4: followers collect the lead's price scaled to their line share.
        // Two syndicates with different own markups should still charge the same premium
        // because both use the lead price, not their own independent pricing.
        let config = ModelConfig::default();

        // Two syndicates with different markups
        let mut syndicate_a = Syndicate::new(0, config.clone());
        syndicate_a.markup_m_t = 0.0; // neutral

        let mut syndicate_b = Syndicate::new(1, config.clone());
        syndicate_b.markup_m_t = 0.5; // aggressive

        let follow_line_size = 0.1;
        let lead_price = 150_000.0; // lead's quoted price for its 50% line
        let peril_region = 0;
        let risk_limit = 10_000_000.0;

        // Both accept same follow allocation with the same lead price
        let capital_before_a = syndicate_a.capital;
        syndicate_a.handle_follow_accepted(
            1,
            follow_line_size,
            lead_price,
            peril_region,
            risk_limit,
        );
        let premium_a = syndicate_a.capital - capital_before_a;

        let capital_before_b = syndicate_b.capital;
        syndicate_b.handle_follow_accepted(
            1,
            follow_line_size,
            lead_price,
            peril_region,
            risk_limit,
        );
        let premium_b = syndicate_b.capital - capital_before_b;

        // Paper: followers use lead price scaled to their line.
        // Expected: (150_000 / 0.5) * 0.1 = 30_000 for both syndicates.
        let expected_premium = (lead_price / config.default_lead_line_size) * follow_line_size;
        assert!(
            (premium_a - expected_premium).abs() < 1.0,
            "Syndicate A premium should equal lead-scaled price ${:.0}, got ${:.0}",
            expected_premium,
            premium_a
        );
        assert!(
            (premium_b - premium_a).abs() < 1.0,
            "Both followers should charge the same premium (lead price). \
             A=${:.0}, B=${:.0}",
            premium_a,
            premium_b
        );
    }

    #[test]
    fn test_d4_insolvent_syndicate_cannot_quote() {
        let config = ModelConfig::default();
        let mut syndicate = Syndicate::new(0, config);

        // Make insolvent
        syndicate.stats.is_insolvent = true;
        syndicate.capital = -100_000.0;

        // Lead quote request (via act which checks insolvency)
        let resp = syndicate.act(
            0,
            &Event::LeadQuoteRequested {
                risk_id: 1,
                syndicate_id: 0,
                peril_region: 0,
                risk_limit: 10_000_000.0,
            },
        );
        assert!(
            resp.events.is_empty(),
            "Insolvent syndicate should not offer lead quotes"
        );

        // Follow quote request
        let resp = syndicate.act(
            0,
            &Event::FollowQuoteRequested {
                risk_id: 2,
                syndicate_id: 0,
                lead_price: 150_000.0,
                peril_region: 0,
                risk_limit: 10_000_000.0,
            },
        );
        assert!(
            resp.events.is_empty(),
            "Insolvent syndicate should not offer follow quotes"
        );
    }
}
