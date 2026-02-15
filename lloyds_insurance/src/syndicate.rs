use crate::{Event, ModelConfig, PolicyParticipation, Stats, SyndicateStats};
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

    // Annual tracking for dividend calculation
    annual_premiums: f64,
    annual_claims: f64,

    // Underwriting markup: exponentially weighted moving average of market conditions
    // m_t captures competitive pressure based on loss experience
    markup_m_t: f64,
}

impl Syndicate {
    pub fn new(syndicate_id: usize, config: ModelConfig) -> Self {
        let initial_capital = config.initial_capital;
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
            markup_m_t: 0.0, // Start at 0 (no markup, e^0 = 1)
        }
    }

    fn calculate_actuarial_price(&self, _risk_id: usize, industry_avg_loss: f64) -> f64 {
        // Simplified actuarial pricing: P̃_t = z·X̄_t + (1-z)·λ'_t·μ'_t
        // where:
        // - industry_avg_loss = λ'_t·μ'_t = yearly_claim_frequency × gamma_mean
        // - loss_history contains CLAIM AMOUNTS (not per-policy losses)
        // - We need to convert claim amounts to per-policy expected loss by multiplying by frequency

        let z = self.config.internal_experience_weight;
        let line_size = self.config.default_lead_line_size;
        let claim_freq = self.config.yearly_claim_frequency;

        // Syndicate's average CLAIM AMOUNT (from loss_history)
        // Then multiply by frequency to get expected loss per policy
        let syndicate_expected_loss = if !self.loss_history.is_empty() {
            // Exponentially weighted moving average of CLAIM AMOUNTS
            let weight = self.config.loss_recency_weight;
            let mut weighted_sum = 0.0;
            let mut total_weight = 0.0;
            for (i, loss) in self.loss_history.iter().rev().enumerate() {
                let w = (1.0 - weight).powi(i as i32);
                weighted_sum += loss * w;
                total_weight += w;
            }
            let avg_claim_amount = weighted_sum / total_weight;
            // Convert to expected loss per policy: E[loss] = P(claim) × E[amount | claim]
            // Note: avg_claim_amount is already the syndicate's line share (from loss_history)
            avg_claim_amount * claim_freq
        } else {
            // No history yet - use industry average
            industry_avg_loss * line_size
        };

        // Industry expected loss per policy (for our line share)
        let industry_expected_loss = industry_avg_loss * line_size;

        // Combine syndicate and industry experience (both are expected loss per policy now)
        let base_price = z * syndicate_expected_loss + (1.0 - z) * industry_expected_loss;

        // Add volatility loading (simplified - using constant for now)
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

    fn handle_lead_quote_request(
        &mut self,
        risk_id: usize,
        current_t: usize,
    ) -> Vec<(usize, Event)> {
        // Use default industry average for now (will be updated with real stats later)
        let industry_avg_loss = self.config.gamma_mean * self.config.yearly_claim_frequency;
        let line_size = self.config.default_lead_line_size;

        // Calculate actuarial price and apply underwriting markup
        let actuarial_price = self.calculate_actuarial_price(risk_id, industry_avg_loss);
        let price = self.apply_underwriting_markup(actuarial_price);

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

    fn handle_lead_accepted(&mut self, risk_id: usize) {
        // Record premium - must match what we quoted
        let industry_avg_loss = self.config.gamma_mean * self.config.yearly_claim_frequency;
        let line_size = self.config.default_lead_line_size;

        // Calculate actuarial price and apply underwriting markup (must match quote)
        let actuarial_price = self.calculate_actuarial_price(risk_id, industry_avg_loss);
        let price = self.apply_underwriting_markup(actuarial_price);

        self.capital += price;
        self.premium_history.push(price);
        self.annual_premiums += price;

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
        _lead_price: f64,
        current_t: usize,
    ) -> Vec<(usize, Event)> {
        // Followers accept the lead's price and offer their line size
        // (In this simplified version, we don't use lead_price for pricing decisions)
        let line_size = self.config.default_follow_line_size;

        vec![(
            current_t,
            Event::FollowQuoteOffered {
                risk_id,
                syndicate_id: self.syndicate_id,
                line_size,
            },
        )]
    }

    fn handle_follow_accepted(&mut self, risk_id: usize, line_size: f64) {
        // Calculate premium for our follow share
        let industry_avg_loss = self.config.gamma_mean * self.config.yearly_claim_frequency;

        // For followers, we use the same pricing logic but with the follow line size
        // (which is passed in, not the default)
        let full_risk_actuarial = self.calculate_actuarial_price(risk_id, industry_avg_loss);
        let full_risk_price = self.apply_underwriting_markup(full_risk_actuarial);

        // Adjust for the actual line size allocated (may be less than requested)
        let price = (full_risk_price / self.config.default_lead_line_size) * line_size;

        self.capital += price;
        self.premium_history.push(price);
        self.annual_premiums += price;

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
        // Update underwriting markup BEFORE checking insolvency
        // Even insolvent syndicates update their market view (though they won't quote)
        self.update_underwriting_markup();

        // Insolvent syndicates don't pay dividends
        if self.stats.is_insolvent {
            self.annual_premiums = 0.0;
            self.annual_claims = 0.0;
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
            }
        }

        // Reset annual counters
        self.annual_premiums = 0.0;
        self.annual_claims = 0.0;
    }

    fn update_underwriting_markup(&mut self) {
        // Update m_t using EWMA: m_t = β · m_{t-1} + (1-β) · signal_t
        // where signal_t = log(loss_ratio_t)
        //
        // This captures competitive pressure:
        // - High loss ratios (>1) → positive signal → m_t increases → higher premiums
        // - Low loss ratios (<1) → negative signal → m_t decreases → lower premiums
        // - Balanced loss ratios (≈1) → signal ≈ 0 → m_t decays toward 0

        if self.annual_premiums > 0.0 {
            let loss_ratio = self.annual_claims / self.annual_premiums;
            let signal = loss_ratio.ln(); // log(loss_ratio)
            let beta = self.config.underwriter_recency_weight;

            // EWMA update
            self.markup_m_t = beta * self.markup_m_t + (1.0 - beta) * signal;
        }
        // If no premiums this year, keep previous m_t unchanged
    }

    fn update_stats(&mut self) {
        self.stats.capital = self.capital;
        self.stats.update_loss_ratio();
        self.stats.update_profit();
        self.stats.markup_m_t = self.markup_m_t;

        // Update exposure by peril region (simplified - would need risk info)
        // For now, just track total exposure
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
            self.handle_year_end();
            self.update_stats();

            // Report capital to market statistics collector
            return Response::events(vec![(
                current_t,
                Event::SyndicateCapitalReported {
                    syndicate_id: self.syndicate_id,
                    capital: self.capital,
                    annual_premiums: self.annual_premiums,
                    annual_claims: self.annual_claims,
                    num_policies: self.stats.num_policies,
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
            } if *syndicate_id == self.syndicate_id => {
                Response::events(self.handle_lead_quote_request(*risk_id, current_t))
            }
            Event::LeadQuoteAccepted {
                risk_id,
                syndicate_id,
            } if *syndicate_id == self.syndicate_id => {
                self.handle_lead_accepted(*risk_id);
                Response::new()
            }
            Event::FollowQuoteRequested {
                risk_id,
                syndicate_id,
                lead_price,
            } if *syndicate_id == self.syndicate_id => {
                Response::events(self.handle_follow_quote_request(*risk_id, *lead_price, current_t))
            }
            Event::FollowQuoteAccepted {
                risk_id,
                syndicate_id,
                line_size,
            } if *syndicate_id == self.syndicate_id => {
                self.handle_follow_accepted(*risk_id, *line_size);
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

        // With no history, should use industry average scaled by line size
        let industry_avg = config.gamma_mean * config.yearly_claim_frequency;
        let line_size = config.default_lead_line_size;
        let base_price = industry_avg * line_size;
        let volatility_loading = config.volatility_weight * base_price;
        let expected_price = base_price + volatility_loading;

        let price = syndicate.calculate_actuarial_price(1, industry_avg);

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

        syndicate.handle_lead_accepted(1);

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
        let quote_events = syndicate.handle_lead_quote_request(1, 0);
        let quoted_price = match &quote_events[0].1 {
            Event::LeadQuoteOffered { price, .. } => *price,
            _ => panic!("Expected LeadQuoteOffered event"),
        };

        // Step 2: Accept that quote
        let initial_capital = syndicate.capital;
        syndicate.handle_lead_accepted(1);

        // Step 3: Verify the premium collected matches the quoted price
        let premium_collected = syndicate.capital - initial_capital;
        assert_eq!(
            premium_collected, quoted_price,
            "Premium collected (${:.0}) should match quoted price (${:.0})",
            premium_collected, quoted_price
        );

        // Also verify stats match
        assert_eq!(syndicate.stats.total_premiums_collected, quoted_price);
    }

    #[test]
    fn test_underwriting_markup_increases_after_losses() {
        let config = ModelConfig::default();
        let mut syndicate = Syndicate::new(0, config.clone());

        // Initial markup should be 0 (no markup)
        assert_eq!(syndicate.markup_m_t, 0.0);

        // Simulate a high-loss year: loss_ratio = 2.0
        syndicate.annual_premiums = 1_000_000.0;
        syndicate.annual_claims = 2_000_000.0;

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
}
