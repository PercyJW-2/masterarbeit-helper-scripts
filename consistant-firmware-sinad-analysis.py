import polars as pl
import argparse

import scipy
import numpy as np
import matplotlib.pyplot as plt
import pysnr

import pywt

parser = argparse.ArgumentParser(
    description="Analysing fast-firmware data and plotting fft"
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
    median_timediff: float,
):
    end_times = [1_000_000, len(data)]
    fig, axs = plt.subplots(2, 2)
    frequency = 1000

    xf_1m = None
    yf_1m = None
    sinad_1m = 0

    for end_check, ax0, ax1, title in zip(
        end_times,
        axs[0],
        axs[1],
        ["1M Samples", "Max Samples"],
    ):
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
        ax0.set(title=f"{title}\nSinad: {sinad:.2f} dB")
        print(f"Sinad {title}: ", sinad, " dB")
    fig.align_labels()
    plt.show()

    if xf_1m is not None and yf_1m is not None:
        plt.semilogy(
            xf_1m[1:], 2.0 / end_times[-1] * np.abs(yf_1m[1 : end_times[-1] // 2])
        )
        plt.title(f"sinad: {sinad_1m:.2f}dB, enob: {(sinad_1m - 1.76) / 6.02:.2f} bits")
        plt.show()


raw_data = (pl.scan_csv(args.path)).collect()
# raw_data = (
#    pl.scan_csv(
#        "~/Dokumente/Waveforms/adc_behind_mux.csv",
#        skip_rows_after_header=2,
#        separator=";",
#        decimal_comma=True,
#    )
# ).collect()

current_df = raw_data.select("Current")
print(
    "max value",
    current_df.max().to_numpy()[0][0],
    "min value",
    current_df.min().to_numpy()[0][0],
)

current_np = current_df.to_numpy()
current_np = np.ravel(current_np)

measurement_indices = raw_data.select("MeasurementIndex").to_numpy().ravel()
index_diffs = measurement_indices[1:] - measurement_indices[0:-1]
artifacts = index_diffs[index_diffs != 1]
artifacts = artifacts[artifacts != 0 - 0xFFFF]
artifact_idx = [
    np.arange(0, len(index_diffs))[index_diffs == artifact] for artifact in artifacts
]
print("measurement artifacts:", artifacts, "\n", artifact_idx)
show_artifact_regions = input("Show Artifact regions? [y/N] ")
if show_artifact_regions == "y":
    for artifact in artifact_idx:
        artifact = artifact[0]
        plt.plot(current_np[artifact - 10 : artifact + 10])
        plt.show()

print(current_np.shape)

sample_rate = int(input("Samplerate in Samples per Second: "))
period = 1 / sample_rate

time = np.linspace(0, (len(current_np) - 1) * period, len(current_np))

# plt.plot(time, current_np)
# plt.title("Raw data")
# plt.show()

check_different_time_scales(current_np, 1 / 8000)

fig, (ax0, ax1) = plt.subplots(2, 1)
ax0.plot(time, current_np)
ax0.set_ylabel("Signal")
ax1.specgram(current_np, NFFT=8192, Fs=period)
ax1.set_xlabel("Time (s)")
# ax1.set_xlim(0, len(current_np) / 8000)
plt.show()

do_bandpass = input("Do Bandpass Filter? [y/N] ")
if do_bandpass == "y":
    frequency = int(input("Frequency to filter: "))
    sos = scipy.signal.butter(
        5, [frequency - 10, frequency + 10], "bandpass", fs=sample_rate, output="sos"
    )
    filtered_data = scipy.signal.sosfilt(sos, current_np)
    plt.plot(filtered_data)
    plt.plot(current_np)
    plt.show()

# exit(0)

wavelet = "cmor1.5-1.0"
widths = np.geomspace(1, 4096, num=80)
sampling_period = period
cwtmatr, freqs = pywt.cwt(
    current_np[3_000_000:5_000_000], widths, wavelet, sampling_period=sampling_period
)
cwtmatr = np.abs(cwtmatr[:-1, :-1])

fig, axs = plt.subplots(2, 1, sharex=True)
pcm = axs[0].pcolormesh(time[3_000_000:5_000_000], freqs, cwtmatr, rasterized=True)
axs[0].set_yscale("log")
axs[0].set_xlabel("Time (s)")
axs[0].set_ylabel("Frequency (Hz)")
axs[0].set_title("Continuous Wavelet Transform (Scaleogram)")
fig.colorbar(pcm, ax=axs[0])

axs[1].plot(time, current_np)
# yf = np.fft.rfft(current_np)
# xf = np.fft.rfftfreq(len(current_np), sampling_period)
# plt.semilogx(xf, np.abs(yf))
# axs[1].set_xlabel("Frequency (Hz")
# axs[1].set_title("Fourier Transform")
plt.tight_layout()
plt.show()
