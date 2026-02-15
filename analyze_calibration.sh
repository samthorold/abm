#!/bin/bash
# Quick VaR calibration analysis script

echo "VaR EM Calibration Analysis"
echo "============================"
echo ""

# Function to analyze final outcomes
analyze_scenario() {
    local scenario=$1
    local label=$2

    echo "$label"
    echo "---"

    local total_insolvent=0
    local total_uniform_dev=0
    local count=0

    for rep in {0..9}; do
        local file="lloyds_insurance/exp3_${scenario}_rep${rep}_time_series.csv"
        if [ -f "$file" ]; then
            local last_line=$(tail -n 1 "$file")
            local insolvent=$(echo "$last_line" | cut -d',' -f6)
            local uniform_dev=$(echo "$last_line" | cut -d',' -f14)

            total_insolvent=$(echo "$total_insolvent + $insolvent" | bc)
            total_uniform_dev=$(echo "$total_uniform_dev + $uniform_dev" | bc)
            count=$((count + 1))
        fi
    done

    if [ $count -gt 0 ]; then
        local avg_insolvent=$(echo "scale=2; $total_insolvent / $count" | bc)
        local avg_uniform=$(echo "scale=4; $total_uniform_dev / $count" | bc)

        echo "  Replications: $count"
        echo "  Avg Insolvent: $avg_insolvent"
        echo "  Avg Uniform Deviation: $avg_uniform"
    else
        echo "  No data found"
    fi
    echo ""
}

# Analyze both scenarios
analyze_scenario "scenario2" "Scenario 2 (No VaR EM)"
analyze_scenario "scenario3" "Scenario 3 (With VaR EM)"

echo "Comparison"
echo "---"
echo "VaR EM improves if Scenario 3 has:"
echo "  - Lower avg insolvent than Scenario 2"
echo "  - Lower avg uniform_deviation than Scenario 2"
