import polars as pl
import argparse

import scipy
from scipy import interpolate
import numpy as np
import matplotlib.pyplot as plt
import pysnr

parser = argparse.ArgumentParser(
    description="Analysing tegrastats-net data and plotting fft"
)
_ = parser.add_argument("path", help="Path to csv file to analyze")

args = parser.parse_args()


def calc_fft(data: np.ndarray, T: float) -> tuple[np.ndarray, np.ndarray, int]:
    N: int = data.shape[0]
    yf = scipy.fftpack.fft(data)
    xf = np.linspace(0, 1 / (2.0 * T), N // 2)
    return xf, yf, N


def check_different_time_scales(
    data: np.ndarray,
    start_time: float,
    end_time: float,
    median_timediff: float,
    average: bool = False,
):
    # data is already resampled to median_time_diff over complete measured time
    # input of 10 kHz -> 100 us per period we check difference between 20, 200, 2000 and full data
    samples_per_period = 200 / median_timediff
    end_times = [
        int(samples_per_period * 20),
        int(samples_per_period * 200),
        int(samples_per_period * 2000),
        1_000_000,
    ]
    fig, axs = plt.subplots(2, 4)
    frequency = 1 / (median_timediff / 1_000_000)

    xf_1m = None
    yf_1m = None
    sinad_1m = 0

    for end_check, ax0, ax1, title in zip(
        end_times,
        axs[0],
        axs[1],
        ["20", "200", "2000", f"Full {int(1_000_000 / samples_per_period)}"],
    ):
        if average:
            xf_list: list[np.ndarray] = []
            yf_list: list[np.ndarray] = []
            sinad_list: list[float] = []
            for section_start in range(data.shape[0] // end_check):
                start_index = section_start * end_check
                data_section = data[start_index : start_index + end_check]
                xff, yff, _ = calc_fft(data_section, median_timediff)
                xf_list.append(xff)
                yf_list.append(yff)
                sinad_s, _ = pysnr.sinad_signal(data_section, frequency)
                sinad_list.append(sinad_s)
            xf = np.mean(np.array(xf_list), axis=0)
            yf = np.mean(np.array(yf_list), axis=0)
            N = data_section.shape[0]
            sinad: float = float(np.nanmean(sinad_list))
        else:
            data_check = data[0:end_check]
            N = data_check.shape[0]
            print(N)
            xf, yf, _ = calc_fft(data_check, median_timediff)
            sinad, _ = pysnr.sinad_signal(data_check, frequency)
            if end_check == end_times[-1]:
                xf_1m = xf
                yf_1m = yf
                sinad_1m = sinad

        ax0.semilogy(xf, 2.0 / N * np.abs(yf[: N // 2]))
        ax1.plot(xf, 2.0 / N * np.abs(yf[: N // 2]))
        ax0.set(title=f"{title} Periods\nSinad: {sinad:.2f} dB")
        print(f"Sinad {title}: ", sinad, " dB")
    fig.align_labels()
    plt.show()

    if xf_1m is not None and yf_1m is not None:
        plt.semilogy(
            xf_1m[1:], 2.0 / end_times[-1] * np.abs(yf_1m[1 : end_times[-1] // 2])
        )
        plt.title(f"sinad: {sinad_1m:.2f}dB, enob: {(sinad_1m - 1.76) / 6.02:.2f} bits")
        plt.show()


raw_data = (pl.scan_csv(args.path).filter(pl.col("Current") != 0)).collect()
time_diffs = raw_data.with_columns(diff=pl.col("MeasurementTime").diff()).select("diff")
minimal_time_diff = time_diffs.min().to_numpy()[0][0]
maximal_time_diff = time_diffs.max().to_numpy()[0][0]
avg_time_diff = time_diffs.mean().to_numpy()[0][0]
median_time_diff = time_diffs.median().to_numpy()[0][0]
std_dev_time = time_diffs.std().to_numpy()[0][0]

print("Time Diffs")
print(
    "min: ",
    minimal_time_diff,
    " max: ",
    maximal_time_diff,
    " avg: ",
    avg_time_diff,
    " median: ",
    median_time_diff,
    " std dev: ",
    std_dev_time,
)

end_time = raw_data.select("MeasurementTime").to_numpy()[-1][0]
start_time = raw_data.select("MeasurementTime").to_numpy()[0][0]
time_range = np.arange(start_time, end_time, median_time_diff)
print("Interpolated Sample Time diff: ", median_time_diff, "us")

current_df = raw_data.select("Current")
print(
    "max value",
    current_df.max().to_numpy()[0][0],
    "min value",
    current_df.min().to_numpy()[0][0],
)

current_and_time = raw_data.select(["MeasurementTime", "Current"]).to_numpy().T

current_interpolated = interpolate.interp1d(
    current_and_time[0], current_and_time[1], kind="linear"
)
y_current_interpolated = current_interpolated(time_range)
# y_current_interpolated = current_and_time[1]

plt.plot(current_and_time[0], current_and_time[1])
plt.title("Raw Data")
plt.show()

plt.plot(time_range, y_current_interpolated)
plt.title("Interpolated Data")
plt.show()

check_different_time_scales(
    y_current_interpolated, start_time, end_time, median_time_diff
)
# check_different_time_scales(current_and_time[1], start_time, end_time, median_time_diff)
# check_different_time_scales(
#    y_current_interpolated, start_time, end_time, median_time_diff, True
# )
