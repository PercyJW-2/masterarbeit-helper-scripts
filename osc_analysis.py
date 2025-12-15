import polars as pl
import argparse

parser = argparse.ArgumentParser(description="Analysing OSC data and plotting fft")
_ = parser.add_argument("path", help="Path to csv file to analyze")

args = parser.parse_args()

raw_data = (
    pl.scan_csv(args.path).unique(
        "MeasurementTimestamp", maintain_order=True, keep="last"
    )
).collect()

frame = raw_data.with_columns(diff=pl.col("MeasurementTimestamp").diff()).select("diff")

print(
    "Max:",
    frame.max().to_numpy(),
    "Min:",
    frame.min().to_numpy(),
    "Median:",
    frame.median().to_numpy(),
)

sample_nums = raw_data.select("SampleIndex")

print(
    "Max:",
    sample_nums.max().to_numpy(),
    "Min:",
    sample_nums.min().to_numpy(),
    "Median",
    sample_nums.median().to_numpy(),
)
