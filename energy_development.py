from typing import Optional
import matplotlib.pyplot as plt
import numpy as np
import numpy.typing as npt


def main():
    firmware_data = np.load("./firmware_power.npy")
    jetson_data = np.load("./jetson.npy")
    osc_data = np.load("./oscilloscope.npy")
    shelly_data = np.load("./shelly.npy")

    osc_idx = np.arange(start=0, stop=osc_data.shape[0]) * (1 / 5_000_000)
    firm_frequency = (osc_data.shape[0] / firmware_data.shape[0]) * (1 / 5_000_000)
    firm_idx = np.arange(start=0, stop=firmware_data.shape[0]) * firm_frequency

    jetson_data[:, 0] -= jetson_data[0, 0]
    shelly_data[:, 0] -= shelly_data[0, 0]

    firmware_hist = energy_samples(firmware_data, True, firm_frequency)
    osc_hist = energy_samples(osc_data, True, 1 / 5_000_000)
    jetson_hist = energy_samples(jetson_data, False)
    shelly_hist = energy_samples(shelly_data, False)

    del osc_data
    del firmware_data

    plt.plot(jetson_data[1:, 0], jetson_hist, label="Jetson Energy")
    plt.plot(shelly_data[1:, 0], shelly_hist, label="Shelly Energy")
    plt.plot(firm_idx[1:], firmware_hist, label="Firmware Energy")
    plt.plot(osc_idx[1::300], osc_hist[::300], label="Osc Energy")
    plt.legend()
    plt.xlabel("Seconds")
    plt.ylabel("Joule")
    plt.show()


def energy_samples(
    data: npt.NDArray,
    const_sampling: bool,
    frequency: Optional[float] = None,
) -> npt.NDArray:
    nrgy_samples = None
    if const_sampling:
        if frequency is None:
            raise AttributeError(
                "data has constant sampling and no frequency is provided"
            )
        nrgy_samples = (data[:-1] + data[1:]) / 2 * frequency
    else:
        time_diffs = data[1:, 0] - data[:-1, 0]
        nrgy_samples = (data[:-1, 1] + data[1:, 1]) / 2 * time_diffs
    return np.cumsum(nrgy_samples)


if __name__ == "__main__":
    main()
