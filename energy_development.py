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

    plt.plot(jetson_data[1:, 0], np.cumsum(jetson_hist), label="Jetson Energy")
    plt.plot(shelly_data[1:, 0], np.cumsum(shelly_hist), label="Shelly Energy")
    plt.plot(firm_idx[1:], np.cumsum(firmware_hist), label="Firmware Energy")
    plt.plot(osc_idx[1::300], np.cumsum(osc_hist)[::300], label="Osc Energy")
    plt.legend()
    plt.xlabel("Seconds")
    plt.ylabel("Joule")
    plt.show()

    print("Calculating Normed firmware")
    firmware_normed = calc_watt_per_second(firmware_hist, firm_idx)
    print("Calculating Normed Oscilloscope")
    osc_normed = calc_watt_per_second(osc_hist, osc_idx)
    print("Calculating Normed Jetson")
    jetson_normed = calc_watt_per_second(jetson_hist, jetson_data[:, 0])
    print("Calculating Normed Shelly")
    shelly_normed = calc_watt_per_second(shelly_hist, shelly_data[:, 0])

    del firmware_hist
    del osc_hist

    plt.plot(jetson_normed, label="Jetson")
    plt.plot(shelly_normed, label="Shelly")
    plt.plot(firmware_normed, label="Firmware")
    plt.plot(osc_normed, label="Oscilloscope")
    plt.legend()
    plt.xlabel("Seconds")
    plt.ylabel("Watt")
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
    return nrgy_samples


def calc_watt_per_second(
    nrgy_samples: npt.NDArray, timestamps: npt.NDArray, timeframe: float = 1
) -> npt.NDArray:
    normed_samples = []
    time_diffs = timestamps[1:] - timestamps[:-1]
    frame_overshoot = 0.0
    frame_position = 0.0
    for nrgy, time_diff in zip(nrgy_samples, time_diffs):
        if time_diff + frame_position > timeframe:
            time_to_fill_frame = timeframe - frame_position
            nrgy_to_fill_frame = nrgy * (time_to_fill_frame / time_diff)
            normed_samples.append((frame_overshoot + nrgy_to_fill_frame))
            rem_diff = time_diff - time_to_fill_frame
            while rem_diff >= timeframe:
                normed_samples.append((nrgy * (timeframe / time_diff)))
                rem_diff -= timeframe
            frame_overshoot = nrgy * (rem_diff / time_diff)
            frame_position = rem_diff
        elif time_diff + frame_position == timeframe:
            normed_samples.append((frame_overshoot + nrgy) / timeframe)
            frame_overshoot = 0.0
            frame_position = 0.0
        else:
            frame_overshoot += nrgy
            frame_position += time_diff
    return np.array(normed_samples)


if __name__ == "__main__":
    main()
