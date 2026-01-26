import pandas as pd
import matplotlib.pyplot as plt

df = pd.read_csv("energy_hist.csv")

osc_energy = df.OscEnergy.to_numpy()
firm_energy = df.FirmwareEnergy.to_numpy()

osc_diffs = osc_energy[1:] - osc_energy[:-1]
firm_diffs = firm_energy[1:] - firm_energy[:-1]

plt.plot(osc_diffs, label="Osc")
plt.plot(firm_diffs, label="Firmware")
plt.legend()
plt.show()
