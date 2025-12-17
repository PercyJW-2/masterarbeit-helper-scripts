import polars as pl
import argparse

import scipy
import numpy as np
import matplotlib.pyplot as plt
import pysnr

import pywt

parser = argparse.ArgumentParser(
    description="Trying to find measurement data artifacts"
)
_ = parser.add_argument("path", help="Path to csv file to analyze")

args = parser.parse_args()

raw_data = (pl.scan_csv(args.path)).collect()

current_df = raw_data.select("Current")
max_current_value = current_df.max().to_numpy()[0][0]
min_current_value = current_df.min().to_numpy()[0][0]
if max_current_value >= 4096 or min_current_value <= 0:
    print("Data has oversteer! :(")
else:
    print("Data has no oversteer! :)")

current_np = current_df.to_numpy().ravel()

measurement_indices = raw_data.select("MeasurementIndex").to_numpy().ravel()
index_diffs = measurement_indices[1:] - measurement_indices[0:-1]
artifact_bools = np.logical_and(index_diffs != 1, index_diffs != 0 - 0xFFFF)
artifacts = index_diffs[artifact_bools]
if artifacts.shape[0] != 0:
    print(artifacts.shape[0], " index artifacts found")
    show_artifact_regions = input("Show Artifact regions? [y/N] ")
    if show_artifact_regions == "y":
        artifact_idx = np.arange(0, index_diffs.shape[0])[artifact_bools]
        for artifact in artifact_idx:
            plt.plot(current_np[artifact - 10 : artifact + 10])
            plt.show()
else:
    print("No index artifacts found")

sample_rate = int(input("Samplerate in Samples per Second: "))
period = 1 / sample_rate
time = np.linspace(0, (len(current_np) - 1) * period, len(current_np))

frequency = int(input("Frequency of function generator: "))
sos_0 = scipy.signal.butter(
    5, [frequency - 10, frequency + 10], "bandpass", fs=sample_rate, output="sos"
)
filtered_data = scipy.signal.sosfilt(sos_0, current_np)
sos_1 = scipy.signal.butter(
    5, [frequency - 1, frequency + 1], "bandstop", fs=sample_rate, output="sos"
)
filtered_data = scipy.signal.sosfilt(sos_1, filtered_data)

artifact_peaks, _ = scipy.signal.find_peaks(
    filtered_data, height=200, distance=sample_rate
)
print(len(artifact_peaks))
print(artifact_peaks)
if artifact_peaks.shape[0] > 1:
    print(f"Found {artifact_peaks.shape[0] - 1} Artifacts")
    for artifact_peak in artifact_peaks[1:]:
        wavelet = "cmor1.5-1.0"
        widths = np.geomspace(1, 4096, num=80)
        min_idx = max(0, artifact_peak - sample_rate)
        max_idx = min(current_np.shape[0], artifact_peak + sample_rate)
        cwt_matr, freqs = pywt.cwt(
            current_np[min_idx:max_idx], widths, wavelet, sampling_period=period
        )
        cwt_matr = np.abs(cwt_matr[:-1, :-1])
        fig, axs = plt.subplots()
        pcm = axs.pcolormesh(time[min_idx:max_idx], freqs, cwt_matr, rasterized=True)
        axs.set_yscale("log")
        axs.set_xlabel("Time (s)")
        axs.set_ylabel("Frequency (Hz)")
        axs.set_title("CWT (Scaleogram)")
        fig.colorbar(pcm, ax=axs)
        plt.tight_layout()
        plt.show()
else:
    print("No Artifacts found!")

freq_artifacts = filtered_data[filtered_data > 200]
print(len(freq_artifacts))

plt.plot(filtered_data)
plt.show()
