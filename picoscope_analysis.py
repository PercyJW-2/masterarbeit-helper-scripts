import polars as pl
import numpy as np


def get_cycle_times(data_frame):
    times = []
    last_voltage = 3.2
    last_time = 0.0
    for i, (time, voltage) in enumerate(zip(*raw_data)):
        if i == 0:
            last_time = time
            last_voltage = voltage
            continue
        if last_voltage > 3.3 / 2 and voltage < 3.3 / 2:
            times.append(abs(abs(last_time) - abs(time)))
            last_time = time
        last_voltage = voltage
    return times


raw_data = (
    pl.scan_csv(
        "~/Dokumente/Waveforms/cs-final-firmware.csv",
        skip_rows_after_header=2,
        separator=";",
        decimal_comma=True,
    )
).collect()

time_diffs = get_cycle_times(raw_data)[1:]

max_time = max(time_diffs)
min_time = min(time_diffs)
std_dev = np.std(time_diffs)
mean = np.mean(time_diffs)
median = np.median(time_diffs)

print(
    "max: ",
    max_time,
    " min_time: ",
    min_time,
    " std_dev: ",
    std_dev,
    " mean: ",
    mean,
    " median: ",
    median,
)
flank_count = len(time_diffs)
print("Flanks: ", flank_count)
