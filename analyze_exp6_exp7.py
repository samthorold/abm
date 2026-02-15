#!/usr/bin/env python3
"""Quick analysis of Experiments 6 and 7"""

import pandas as pd
import numpy as np

print("="*60)
print("EXPERIMENT 6 & 7: QUICK ANALYSIS")
print("="*60)
print()

# Experiment 6: Markup Mechanism
print("EXPERIMENT 6: Markup Mechanism Validation")
print("-" * 60)

exp6_data = []
for rep in range(10):
    df = pd.read_csv(f'lloyds_insurance/exp6_rep{rep}_time_series.csv')
    exp6_data.append(df)

# Check markup evolution
mean_df = pd.concat(exp6_data).groupby('year').mean()
print("\nMarkup evolution (every 10 years):")
print("Year  Avg Markup  Std Dev")
for year in [0, 10, 20, 30, 40, 49]:
    avg_markup = mean_df.loc[year, 'markup_avg']
    std_markup = mean_df.loc[year, 'markup_std_dev']
    print(f"{year:4d}  {avg_markup:10.4f}  {std_markup:8.4f}")

# Check loss ratios
print("\nLoss ratio evolution:")
print("Year  Avg Loss Ratio")
for year in [0, 10, 20, 30, 40, 49]:
    avg_lr = mean_df.loc[year, 'avg_loss_ratio']
    print(f"{year:4d}  {avg_lr:14.4f}")

# Experiment 7: Loss Coupling
print()
print("EXPERIMENT 7: Loss Coupling")
print("-" * 60)

exp7_data = []
for rep in range(10):
    df = pd.read_csv(f'lloyds_insurance/exp7_rep{rep}_time_series.csv')
    exp7_data.append(df)

mean_df7 = pd.concat(exp7_data).groupby('year').mean()

print("\nMarket evolution (every 10 years):")
print("Year  Insolvent  Loss Ratio")
for year in [0, 10, 20, 30, 40, 49]:
    insol = mean_df7.loc[year, 'num_insolvent_syndicates']
    lr = mean_df7.loc[year, 'avg_loss_ratio']
    print(f"{year:4d}  {insol:9.2f}  {lr:10.4f}")

# Overall Assessment
print()
print("="*60)
print("CRITICAL ASSESSMENT")
print("="*60)
print()

# Check for market collapse indicators
final_insolvent = []
final_loss_ratios = []

for experiments, name in [(exp6_data, "Exp 6"), (exp7_data, "Exp 7")]:
    for df in experiments:
        final_insolvent.append(df.iloc[-1]['num_insolvent_syndicates'])
        final_loss_ratios.append(df.iloc[-1]['avg_loss_ratio'])

print(f"Final insolvencies (avg): {np.mean(final_insolvent):.2f}")
print(f"Final loss ratios (avg): {np.mean(final_loss_ratios):.4f}")
print()

if np.mean(final_insolvent) > 4:
    print("⚠️  HIGH INSOLVENCY RATE - most syndicates failing")
if np.mean(final_loss_ratios) < 0.5:
    print("⚠️  LOW LOSS RATIOS - suggests insolvent syndicates not paying claims")

print()
print("="*60)
