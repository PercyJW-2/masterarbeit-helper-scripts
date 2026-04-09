from pathlib import Path
from typing import Optional
import matplotlib.pyplot as plt
import numpy as np
import sys


def main(
    firm_samplerate: float,
    osc_samplerate: float,
    storage_path: Path | str,
    firm_start_stop_idx: Optional[tuple[int, int]] = None,
    osc_start_stop_idx: Optional[tuple[int, int]] = None,
    jetson_start_stop_idx: Optional[tuple[int, int]] = None,
    shelly_start_stop_idx: Optional[tuple[int, int]] = None,
) -> None:
    _, ax = plt.subplots()

    if not isinstance(storage_path, Path):
        storage_path = Path(storage_path)

    osc_path = storage_path / "oscilloscope.npy"
    if osc_path.exists():
        osc_power = np.load(osc_path)
        osc_idx = np.arange(start=0, stop=osc_power.shape[0]) * (1 / osc_samplerate)
        osc_samplecount = osc_power.shape[0]
        skip_count = int(osc_samplecount / 1_000_000)
        if skip_count < 1:
            skip_count = 1
        ax.plot(osc_idx[::skip_count], osc_power[::skip_count], label="Osc", color="b")
        if osc_start_stop_idx is not None:
            osc_start = osc_idx[osc_start_stop_idx[0]]
            osc_end = osc_idx[osc_start_stop_idx[1]]
            ax.vlines(
                [osc_start, osc_end],
                0,
                1,
                transform=ax.get_xaxis_transform(),
                colors="b",
            )

    firm_path = storage_path / "firmware_power.npy"
    if firm_path.exists():
        firm_power = np.load("./firmware_power.npy")
        firm_idx = np.arange(start=0, stop=firm_power.shape[0]) * (1 / firm_samplerate)
        ax.plot(firm_idx, firm_power, label="Firmware", color="orange")
        if firm_start_stop_idx is not None:
            firm_start = firm_idx[firm_start_stop_idx[0]]
            firm_end = firm_idx[firm_start_stop_idx[1]]
            ax.vlines(
                [firm_start, firm_end],
                0,
                1,
                transform=ax.get_xaxis_transform(),
                colors="orange",
            )

    jetson_path = storage_path / "jetson.npy"
    if jetson_path.exists():
        jetson_power = np.load("./jetson.npy")
        jetson_power[:, 0] -= jetson_power[0, 0]
        ax.plot(*jetson_power.T, label="Jetson", color="g")
        if jetson_start_stop_idx is not None:
            jetson_start = jetson_power[jetson_start_stop_idx[0], 0]
            jetson_end = jetson_power[jetson_start_stop_idx[1], 0]
            ax.vlines(
                [jetson_start, jetson_end],
                0,
                1,
                transform=ax.get_xaxis_transform(),
                colors="g",
            )

    shelly_path = storage_path / "shelly.npy"
    if shelly_path.exists():
        shelly_power = np.load("./shelly.npy")
        shelly_power[:, 0] -= shelly_power[0, 0]
        ax.plot(*shelly_power.T, label="Shelly", color="r")
        if shelly_start_stop_idx is not None:
            shelly_start = shelly_power[shelly_start_stop_idx[0], 0]
            shelly_end = shelly_power[shelly_start_stop_idx[1], 0]
            ax.vlines(
                [shelly_start, shelly_end],
                0,
                1,
                transform=ax.get_xaxis_transform(),
                colors="r",
            )

    plt.legend(loc="lower center")
    plt.ylabel("Watt")
    plt.xlabel("Seconds")
    plt.show()


if __name__ == "__main__":
    firm_samplerate = float(sys.argv[1])
    osc_samplerate = float(sys.argv[2])
    main(firm_samplerate, osc_samplerate, Path("./"))
