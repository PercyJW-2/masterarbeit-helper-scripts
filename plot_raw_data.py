import polars as pl
import numpy as np
import matplotlib.pyplot as plt
import sys

if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("Please provide path")
        exit(-1)
    path = sys.argv[1]

    pico_data = pl.read_parquet(path + "/usb_osc_data.parquet")
    firmware_data = pl.read_parquet(path + "/fast_firmware.parquet")

    osc_idx = np.arange(start=0, stop=pico_data.shape[0]) * (1 / 2000)
    firm_idx = np.arange(start=0, stop=firmware_data.shape[0]) * (1 / 2000)

    plt.plot(osc_idx, pico_data["current"] * 2, label="osc")
    plt.plot(firm_idx, firmware_data["current"] / 1000, label="firmware")

    plt.legend(loc="lower center")
    plt.xlabel("seconds")
    plt.ylabel("amp (not calibrated)")
    plt.show()
