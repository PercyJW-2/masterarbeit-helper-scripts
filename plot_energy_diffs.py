import matplotlib.pyplot as plt
import numpy as np
import sys

firm_samplerate = float(sys.argv[1])

osc_power = np.load("./oscilloscope.npy")
firm_power = np.load("./firmware_power.npy")

osc_idx = np.arange(start=0, stop=osc_power.shape[0]) * (1 / 2000)
firm_idx = np.arange(start=0, stop=firm_power.shape[0]) * (1 / firm_samplerate)

plt.plot(osc_idx, osc_power, label="Osc")
plt.plot(firm_idx, firm_power, label="Firmware")
plt.legend()
plt.show()
