import polars as pl
from pysnr import sinad_signal

data_frame_12_bit = (
    pl.scan_csv("/media/jwachsmuth/USBSTICK/sinad/messung12bit.csv", skip_lines=13)
).collect()
data_frame_16_bit = (
    pl.scan_csv("/media/jwachsmuth/USBSTICK/sinad/messung16bit.csv", skip_lines=13)
).collect()

print("Loaded both csv files")

sinad_12bit, _ = sinad_signal(data_frame_12_bit["CH1"].to_numpy(), 1 / 2e-7)
sinad_16bit, _ = sinad_signal(data_frame_16_bit["CH1"].to_numpy(), 1 / 2e-7)

print("sinad 12 bit: ", sinad_12bit)
print("sinad 16 bit: ", sinad_16bit)
