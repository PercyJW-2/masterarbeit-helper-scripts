import matplotlib.pyplot as plt
import numpy as np
import sys


def main(firm_samplerate, osc_samplerate):
    osc_power = np.load("./oscilloscope.npy")
    firm_power = np.load("./firmware_power.npy")
    jetson_power = np.load("./jetson.npy")
    shelly_power = np.load("./shelly.npy")

    osc_idx = np.arange(start=0, stop=osc_power.shape[0]) * (1 / osc_samplerate)
    firm_idx = np.arange(start=0, stop=firm_power.shape[0]) * (1 / firm_samplerate)

    print("Duration: ", osc_idx[-1])

    jetson_power[:, 0] -= jetson_power[0, 0]
    shelly_power[:, 0] -= shelly_power[0, 0]

    osc_samplecount = osc_power.shape[0]
    skip_count = int(osc_samplecount / 1_000_000)
    if skip_count < 1:
        skip_count = 1

    plt.plot(osc_idx[::skip_count], osc_power[::skip_count], label="Osc")
    plt.plot(firm_idx, firm_power, label="Firmware")
    plt.plot(*jetson_power.T, label="Jetson")
    plt.plot(*shelly_power.T, label="Shelly")
    plt.legend(loc="lower center")
    plt.ylabel("Watt")
    plt.xlabel("Seconds")
    plt.show()


if __name__ == "__main__":
    firm_samplerate = float(sys.argv[1])
    osc_samplerate = float(sys.argv[2])
