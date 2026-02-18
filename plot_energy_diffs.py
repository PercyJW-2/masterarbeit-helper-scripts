import matplotlib.pyplot as plt
import numpy as np
import sys

firm_samplerate = float(sys.argv[1])
osc_samplerate = float(sys.argv[2])

osc_power = np.load("./oscilloscope.npy")
firm_power = np.load("./firmware_power.npy")
jetson_power = np.load("./jetson.npy")
shelly_power = np.load("./shelly.npy")

osc_idx = np.arange(start=0, stop=osc_power.shape[0]) * (1 / osc_samplerate)
firm_idx = np.arange(start=0, stop=firm_power.shape[0]) * (1 / firm_samplerate)

jetson_power[:, 0] -= jetson_power[0, 0]
shelly_power[:, 0] -= shelly_power[0, 0]

plt.plot(osc_idx, osc_power, label="Osc")
plt.plot(firm_idx, firm_power, label="Firmware")
plt.plot(*jetson_power.T, label="Jetson")
plt.plot(*shelly_power.T, label="Shelly")
plt.legend(loc="lower center")
plt.ylabel("Watt")
plt.xlabel("Seconds")
plt.show()
