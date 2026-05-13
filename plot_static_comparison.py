from textwrap import fill
from matplotlib.axes import Axes
from matplotlib.figure import Figure
import matplotlib.pyplot as plt
from pandas.plotting import boxplot
import yaml
import argparse
import numpy as np
from pathlib import Path

parser = argparse.ArgumentParser(
    "Plot static comparison measurement done with measurement suite"
)
parser.add_argument("-p", "--path", help="Root folder of measurement", required=True)

type Duration = float
type Energy = float


def load_all_yaml(
    path: Path,
) -> tuple[
    list[Duration],
    list[Duration],
    list[Duration],
    list[Duration],
    list[Energy],
    list[Energy],
    list[Energy],
    list[Energy],
]:
    pico_data = []
    urecs_data = []
    shelly_data = []
    jetson_data = []
    pico_energy = []
    urecs_energy = []
    shelly_energy = []
    jetson_energy = []
    for folder in [x for x in path.iterdir() if x.is_dir()]:
        with (folder / "results.yaml").open() as result_file:
            result = yaml.safe_load(result_file)
            pico_data.append(result["oscilloscope_results"]["results"]["duration"])
            pico_energy.append(result["oscilloscope_results"]["results"]["energy"])
            urecs_data.append(result["firmware_results"]["duration"])
            urecs_energy.append(result["firmware_results"]["energy"])
            shelly_data.append(result["shelly_results"]["duration"])
            shelly_energy.append(result["shelly_results"]["energy"])
            if result["jetson_results"] is not None:
                jetson_data.append(result["jetson_results"]["duration"])
                jetson_energy.append(result["jetson_results"]["energy"])
    return (
        pico_data,
        urecs_data,
        shelly_data,
        jetson_data,
        pico_energy,
        urecs_energy,
        shelly_energy,
        jetson_energy,
    )


if __name__ == "__main__":
    args = parser.parse_args()
    pico, urecs, shelly, jetson, pico_e, urecs_e, shelly_e, jetson_e = load_all_yaml(
        Path(args.path)
    )
    pico_w = [e / d for (e, d) in zip(pico_e, pico)]
    urecs_w = [e / d for (e, d) in zip(urecs_e, urecs)]
    shelly_w = [e / d for (e, d) in zip(shelly_e, shelly)]
    jetson_w = [e / d for (e, d) in zip(jetson_e, jetson)]

    ret: tuple[Figure, np.ndarray] = plt.subplots(1, 2)
    fig, axs = ret
    axs_l: list[Axes] = list(axs.ravel())
    fig.set_size_inches((15, 10))

    if len(jetson_w) > 0:
        axs_l[0].boxplot(
            [pico_w, urecs_w, shelly_w, jetson_w],
            showfliers=False,
            tick_labels=["Picoscope", "u.RECS", "Shelly", "Jetson"],
        )
    else:
        axs_l[0].boxplot(
            [pico_w, urecs_w, shelly_w],
            showfliers=False,
            tick_labels=["Picoscope", "u.RECS", "Shelly"],
        )
    axs_l[0].tick_params("x", rotation=90)
    axs_l[0].set_ylabel("Energy per Second (J/s)")
    axs_l[0].set_title("Power Overview")

    pico_median = np.median(pico_w)
    urecs_median = np.median(urecs_w)
    shelly_median = np.median(shelly_w)
    jetson_median = 0
    if len(jetson_w) > 0:
        jetson_median = np.median(jetson_w)

    urecs_diff = 100 * (urecs_median - pico_median) / pico_median
    shelly_diff = 100 * (shelly_median - pico_median) / pico_median
    jetson_diff = 100 * (jetson_median - pico_median) / pico_median
    print(urecs_diff)

    if jetson_median > 0:
        axs_l[1].bar(
            [1, 2, 3], [urecs_diff, shelly_diff, jetson_diff], hatch="////", fill=False
        )
        axs_l[1].set_xticks([1, 2, 3], ["u.RECS", "Shelly", "Jetson"])
    else:
        axs_l[1].bar([1, 2], [urecs_diff, shelly_diff], hatch="////", fill=False)
        axs_l[1].set_xticks([1, 2], ["u.RECS", "Shelly"])
    axs_l[1].tick_params("x", rotation=90)
    axs_l[1].set_title("Difference to Picoscope Results")
    axs_l[1].set_ylabel("Percent (%)")

    fig.tight_layout()

    plt.show()
