import os

import polars as pl
import scipy.signal as signal
import numpy as np


def calc_avg_power(directory: str) -> tuple[float, float, float, float, float, float]:
    fast_firmware_current_data = (
        pl.scan_parquet(directory + "/fast_firmware.parquet").select("current")
    ).collect()
    # sos = signal.butter(10, 750, "low", fs=2000, output="sos")
    # fast_firmware_current = np.mean(
    #    signal.sosfilt(sos, fast_firmware_current_data["Current"])
    # )
    fast_firmware_current = np.mean(
        fast_firmware_current_data["current"].to_numpy()[1:]
    )
    print("Fast Firmware Current:", fast_firmware_current)
    shelly_plug_current = (
        pl.scan_parquet(directory + "/shellyPlug.parquet").select("current").mean()
    ).collect()["current"][0]
    shelly_plug_voltage = (
        pl.scan_parquet(directory + "/shellyPlug.parquet").select("voltage").mean()
    ).collect()["voltage"][0]
    shelly_plug_power = (
        pl.scan_parquet(directory + "/shellyPlug.parquet").select("power").mean()
    ).collect()["power"][0]
    print(
        "Shelly Current:",
        shelly_plug_current,
        " Voltage:",
        shelly_plug_voltage,
        " Power:",
        shelly_plug_power,
    )
    pico_voltage = (
        pl.scan_parquet(directory + "/usb_osc_data.parquet").select("voltage").mean()
    ).collect(engine="streaming")["voltage"][0]
    pico_current = (
        pl.scan_parquet(directory + "/usb_osc_data.parquet").select("current").mean()
    ).collect(engine="streaming")["current"][0]
    print("Pico Current:", pico_current, " Voltage:", pico_voltage)
    return (
        float(fast_firmware_current),
        shelly_plug_current,
        shelly_plug_voltage,
        shelly_plug_power,
        pico_current,
        pico_voltage,
    )


folder = os.scandir(
    "/mnt/6e97041d-abf4-4100-8bef-9111a0c14742/power_measurements/static_analysis/"
)

values = {}
current_vals = []

for f in folder:
    print(f.name)
    calc_values = calc_avg_power(f.path)
    current = int(f.name[:-2])
    values[current] = calc_values
    current_vals.append(current)

current_vals.sort()
df_dict = {
    "actual_value": [],
    "firmware_current": [],
    "shelly_current": [],
    "shelly_voltage": [],
    "shelly_power": [],
    "pico_current": [],
    "pico_voltage": [],
}
for current in current_vals:
    calc_values = values[current]
    df_dict["actual_value"].append(current)
    df_dict["firmware_current"].append(calc_values[0])
    df_dict["shelly_current"].append(calc_values[1])
    df_dict["shelly_voltage"].append(calc_values[2])
    df_dict["shelly_power"].append(calc_values[3])
    df_dict["pico_current"].append(calc_values[4])
    df_dict["pico_voltage"].append(calc_values[5])

df = pl.DataFrame(df_dict)

df.write_csv("calc_values.csv")
