/// Market configuration with supply and demand schedules
#[derive(Debug, Clone)]
pub struct MarketConfig {
    pub id: usize,
    pub name: String,
    pub equilibrium_price: usize,
    pub equilibrium_quantity: usize,

    /// Buyer values: outer vec = buyers (6 total), inner vec = units per buyer
    pub buyer_values: Vec<Vec<usize>>,

    /// Seller costs: outer vec = sellers (6 total), inner vec = units per seller
    pub seller_costs: Vec<Vec<usize>>,
}

impl MarketConfig {
    /// Calculate maximum possible surplus (competitive equilibrium surplus)
    /// Sort all values descending, all costs ascending
    /// Match until marginal value < marginal cost
    pub fn calculate_max_surplus(&self) -> i32 {
        // Flatten all buyer values
        let mut all_values: Vec<usize> = self
            .buyer_values
            .iter()
            .flat_map(|buyer| buyer.iter().copied())
            .collect();
        all_values.sort_by(|a, b| b.cmp(a)); // Descending

        // Flatten all seller costs
        let mut all_costs: Vec<usize> = self
            .seller_costs
            .iter()
            .flat_map(|seller| seller.iter().copied())
            .collect();
        all_costs.sort(); // Ascending

        let mut surplus = 0i32;
        for (value, cost) in all_values.iter().zip(all_costs.iter()) {
            if value >= cost {
                surplus += (*value as i32) - (*cost as i32);
            } else {
                break;
            }
        }

        surplus
    }

    /// Get total number of buyers
    pub fn num_buyers(&self) -> usize {
        self.buyer_values.len()
    }

    /// Get total number of sellers
    pub fn num_sellers(&self) -> usize {
        self.seller_costs.len()
    }

    /// Market 1: Standard design with equilibrium price ≈ 69
    /// Values from paper Table (reconstructed from figures)
    pub fn market_1() -> Self {
        let buyer_values = vec![
            vec![150, 100, 70, 50],
            vec![145, 95, 68, 45],
            vec![140, 90, 66, 40],
            vec![135, 85, 64, 35],
            vec![130, 80, 62, 30],
            vec![125, 75, 60, 25],
        ];

        let seller_costs = vec![
            vec![10, 45, 65, 95],
            vec![15, 50, 67, 100],
            vec![20, 55, 69, 105],
            vec![25, 60, 71, 110],
            vec![30, 62, 73, 115],
            vec![35, 64, 75, 120],
        ];

        MarketConfig {
            id: 1,
            name: "Market 1".to_string(),
            equilibrium_price: 69,
            equilibrium_quantity: 15, // Approximate
            buyer_values,
            seller_costs,
        }
    }

    /// Market 2: Different curve shapes, same equilibrium ≈ 69
    /// Design: Flatter supply, steeper demand
    pub fn market_2() -> Self {
        let buyer_values = vec![
            vec![160, 90, 70, 30],
            vec![155, 88, 69, 28],
            vec![150, 86, 68, 26],
            vec![145, 84, 67, 24],
            vec![140, 82, 66, 22],
            vec![135, 80, 65, 20],
        ];

        let seller_costs = vec![
            vec![15, 50, 68, 85],
            vec![18, 52, 69, 87],
            vec![21, 54, 70, 89],
            vec![24, 56, 71, 91],
            vec![27, 58, 72, 93],
            vec![30, 60, 73, 95],
        ];

        MarketConfig {
            id: 2,
            name: "Market 2".to_string(),
            equilibrium_price: 69,
            equilibrium_quantity: 15,
            buyer_values,
            seller_costs,
        }
    }

    /// Market 3: Low volume market with equilibrium price ≈ 106, quantity = 6
    /// Small total surplus available
    pub fn market_3() -> Self {
        let buyer_values = vec![
            vec![120, 106],
            vec![118, 105],
            vec![116, 104],
            vec![114, 102],
            vec![112, 100],
            vec![110, 98],
        ];

        let seller_costs = vec![
            vec![95, 107],
            vec![96, 108],
            vec![97, 109],
            vec![98, 110],
            vec![99, 111],
            vec![100, 112],
        ];

        MarketConfig {
            id: 3,
            name: "Market 3".to_string(),
            equilibrium_price: 106,
            equilibrium_quantity: 6,
            buyer_values,
            seller_costs,
        }
    }

    /// Market 4: High equilibrium price ≈ 170
    /// Tests whether results hold when equilibrium is in upper portion of price range
    pub fn market_4() -> Self {
        let buyer_values = vec![
            vec![195, 185, 175, 165],
            vec![193, 183, 173, 163],
            vec![191, 181, 171, 161],
            vec![189, 179, 169, 159],
            vec![187, 177, 167, 157],
            vec![185, 175, 165, 155],
        ];

        let seller_costs = vec![
            vec![150, 160, 170, 180],
            vec![152, 162, 171, 181],
            vec![154, 164, 172, 182],
            vec![156, 166, 173, 183],
            vec![158, 168, 174, 184],
            vec![159, 169, 175, 185],
        ];

        MarketConfig {
            id: 4,
            name: "Market 4".to_string(),
            equilibrium_price: 170,
            equilibrium_quantity: 15,
            buyer_values,
            seller_costs,
        }
    }

    /// Market 5: High volume (24 units) with equilibrium price ≈ 131
    /// Crossing curves with marginal units clustered near equilibrium
    /// This makes it difficult to achieve 100% efficiency
    pub fn market_5() -> Self {
        let buyer_values = vec![
            vec![180, 150, 135, 133, 131, 129],
            vec![178, 148, 134, 132, 131, 128],
            vec![176, 146, 133, 132, 130, 127],
            vec![174, 144, 133, 131, 130, 126],
            vec![172, 142, 132, 131, 129, 125],
            vec![170, 140, 132, 130, 129, 124],
        ];

        let seller_costs = vec![
            vec![80, 110, 128, 130, 132, 134],
            vec![82, 112, 128, 130, 132, 135],
            vec![84, 114, 129, 131, 133, 136],
            vec![86, 116, 129, 131, 133, 137],
            vec![88, 118, 130, 132, 134, 138],
            vec![90, 120, 130, 132, 134, 139],
        ];

        MarketConfig {
            id: 5,
            name: "Market 5".to_string(),
            equilibrium_price: 131,
            equilibrium_quantity: 24,
            buyer_values,
            seller_costs,
        }
    }

    /// Get all standard market configurations
    pub fn all_markets() -> Vec<Self> {
        vec![
            Self::market_1(),
            Self::market_2(),
            Self::market_3(),
            Self::market_4(),
            Self::market_5(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_market_1_configuration() {
        let m = MarketConfig::market_1();
        assert_eq!(m.num_buyers(), 6);
        assert_eq!(m.num_sellers(), 6);
        assert!(m.calculate_max_surplus() > 0);
    }

    #[test]
    fn test_all_markets_have_positive_surplus() {
        for market in MarketConfig::all_markets() {
            let surplus = market.calculate_max_surplus();
            assert!(
                surplus > 0,
                "{} should have positive surplus, got {}",
                market.name,
                surplus
            );
        }
    }

    #[test]
    fn test_equilibrium_quantity_matches_calculation() {
        let m = MarketConfig::market_1();

        // Flatten and sort
        let mut values: Vec<usize> = m.buyer_values.iter().flatten().copied().collect();
        values.sort_by(|a, b| b.cmp(a));

        let mut costs: Vec<usize> = m.seller_costs.iter().flatten().copied().collect();
        costs.sort();

        // Count profitable trades
        let eq_qty = values
            .iter()
            .zip(costs.iter())
            .filter(|(v, c)| v >= c)
            .count();

        // Should be close to declared equilibrium quantity
        let diff = (eq_qty as i32 - m.equilibrium_quantity as i32).abs();
        assert!(
            diff <= 2,
            "Market 1 equilibrium quantity should be close to calculated value"
        );
    }

    #[test]
    fn test_market_3_has_correct_unit_counts() {
        let m = MarketConfig::market_3();

        // Market 3 should be low volume: 6 buyers × 2 units each
        let buyer_unit_count: usize = m.buyer_values.iter().map(|units| units.len()).sum();
        let seller_unit_count: usize = m.seller_costs.iter().map(|units| units.len()).sum();

        assert_eq!(buyer_unit_count, 12, "Market 3 should have 12 buyer units");
        assert_eq!(
            seller_unit_count, 12,
            "Market 3 should have 12 seller units"
        );
        assert_eq!(m.num_buyers(), 6);
        assert_eq!(m.num_sellers(), 6);
    }

    #[test]
    fn test_market_5_has_correct_unit_counts() {
        let m = MarketConfig::market_5();

        // Market 5 should be high volume: 6 buyers × 6 units each
        let buyer_unit_count: usize = m.buyer_values.iter().map(|units| units.len()).sum();
        let seller_unit_count: usize = m.seller_costs.iter().map(|units| units.len()).sum();

        assert_eq!(buyer_unit_count, 36, "Market 5 should have 36 buyer units");
        assert_eq!(
            seller_unit_count, 36,
            "Market 5 should have 36 seller units"
        );
        assert_eq!(m.num_buyers(), 6);
        assert_eq!(m.num_sellers(), 6);
    }

    #[test]
    fn test_market_3_equilibrium_quantity_calculation() {
        let m = MarketConfig::market_3();

        let mut values: Vec<usize> = m.buyer_values.iter().flatten().copied().collect();
        values.sort_by(|a, b| b.cmp(a));

        let mut costs: Vec<usize> = m.seller_costs.iter().flatten().copied().collect();
        costs.sort();

        let eq_qty = values
            .iter()
            .zip(costs.iter())
            .filter(|(v, c)| v >= c)
            .count();

        assert_eq!(
            eq_qty, m.equilibrium_quantity,
            "Market 3 calculated equilibrium should match declared (low volume market)"
        );
    }

    #[test]
    fn test_market_5_marginal_units_near_equilibrium() {
        let m = MarketConfig::market_5();

        // Market 5 design feature: marginal units clustered near equilibrium price (131)
        let all_values: Vec<usize> = m.buyer_values.iter().flatten().copied().collect();
        let all_costs: Vec<usize> = m.seller_costs.iter().flatten().copied().collect();

        // Count units within ±5 of equilibrium price
        let marginal_values = all_values
            .iter()
            .filter(|&&v| (126..=136).contains(&v))
            .count();
        let marginal_costs = all_costs
            .iter()
            .filter(|&&c| (126..=136).contains(&c))
            .count();

        assert!(
            marginal_values >= 10,
            "Market 5 should have many buyer values near equilibrium, found {}",
            marginal_values
        );
        assert!(
            marginal_costs >= 10,
            "Market 5 should have many seller costs near equilibrium, found {}",
            marginal_costs
        );
    }

    #[test]
    fn test_extramarginal_units_exist() {
        for market in MarketConfig::all_markets() {
            let mut values: Vec<usize> = market.buyer_values.iter().flatten().copied().collect();
            values.sort_by(|a, b| b.cmp(a));

            let mut costs: Vec<usize> = market.seller_costs.iter().flatten().copied().collect();
            costs.sort();

            // Find first unprofitable pair
            let extramarginal_exists = values.iter().zip(costs.iter()).any(|(v, c)| v < c);

            assert!(
                extramarginal_exists,
                "{} should have extramarginal units (value < cost)",
                market.name
            );
        }
    }

    #[test]
    fn test_market_5_equilibrium_calculation() {
        let m = MarketConfig::market_5();

        let mut values: Vec<usize> = m.buyer_values.iter().flatten().copied().collect();
        values.sort_by(|a, b| b.cmp(a));

        let mut costs: Vec<usize> = m.seller_costs.iter().flatten().copied().collect();
        costs.sort();

        let eq_qty = values
            .iter()
            .zip(costs.iter())
            .filter(|(v, c)| v >= c)
            .count();

        // Should be close to declared
        let diff = (eq_qty as i32 - m.equilibrium_quantity as i32).abs();
        assert!(
            diff <= 2,
            "Market 5 equilibrium quantity should match calculation, expected {}, got {}",
            m.equilibrium_quantity,
            eq_qty
        );
    }

    #[test]
    fn test_all_markets_have_equal_buyers_and_sellers() {
        for market in MarketConfig::all_markets() {
            assert_eq!(
                market.num_buyers(),
                market.num_sellers(),
                "{} should have equal buyers and sellers",
                market.name
            );
        }
    }
}
