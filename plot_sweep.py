from typing import Dict
import matplotlib.pyplot as plt
import yaml
import argparse
from pathlib import Path
import numpy as np

parser = argparse.ArgumentParser("Plot samplerate sweep done with measurement suite")

parser.add_argument("-p", "--path", help="Root folder of msmt sweep", required=True)
parser.add_argument(
    "--constant_cut",
    help="Do calculation manually while keeping the measurement duration constant",
    action="store_true",
)

if __name__ == "__main__":
    args = parser.parse_args()
    path = Path(args.path)

    results: Dict[int, tuple[list[float], list[float]]] = dict()
    for folder in [x for x in path.iterdir() if x.is_dir()]:
        samplerate = int(folder.name[:-3])
        energy = []
        duration = []
        for run in [x for x in folder.iterdir() if x.is_dir()]:
            if args.constant_cut:
                data = np.load((run / "oscilloscope.npy").as_posix())
                samples = int(60 / (1 / samplerate))
                print(
                    f"samplerate {samplerate}\tsamples {samples}\tduration {samples / samplerate}"
                )
                data = data[: int(60 / (1 / samplerate))]
                nrg = np.sum(((data[:-1] + data[1:]) / 2) * (1 / samplerate))
                energy.append(nrg)
                duration.append(samples / samplerate)
                del data
            else:
                result_path = run / "results.yaml"
                if not result_path.exists():
                    continue
                with result_path.open() as result_file:
                    result = yaml.safe_load(result_file)["oscilloscope_results"][
                        "results"
                    ]
                    energy.append(result["energy"])
                    duration.append(result["duration"])
        results[samplerate] = (energy, duration)
    samplerates = list(results.keys())
    samplerates.sort()
    energies = []
    durations = []
    normalized_energies = []
    for samplerate in samplerates:
        energy, duration = results[samplerate]
        normalized_energy = [e / d for e, d in zip(energy, duration)]
        energies.append(energy)
        durations.append(duration)
        normalized_energies.append(normalized_energy)

    fig, axs = plt.subplots(1, 3)
    axs[0].boxplot(energies)
    axs[0].set_ylabel("Energy (J)")
    axs[1].boxplot(durations)
    axs[1].set_ylabel("Duration (s)")
    axs[2].boxplot(normalized_energies)
    axs[2].set_ylabel("Watt (J/s)")
    for ax in axs:
        ax.tick_params("x", rotation=90)
        ax.xaxis.grid(True)
        ax.yaxis.grid(True)
        ax.set_xticks(np.arange(1, len(samplerates) + 1), labels=samplerates)
        ax.set_xlabel("Samplerate")
    plt.show()
